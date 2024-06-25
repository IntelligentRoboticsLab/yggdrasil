use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use itertools::Itertools;

use crate::{
    storage::{BoxedSystem, Storage},
    system::{IntoSystem, NormalSystem},
};

use miette::{miette, Result};
use petgraph::{algo::toposort, prelude::NodeIndex, stable_graph::StableDiGraph, Direction};

const DEFAULT_ORDER_INDEX: u8 = (256u32 / 2u32) as u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemIndex(usize);

#[derive(Default)]
struct Dag {
    graph: StableDiGraph<SystemIndex, (), usize>,
}

impl Dag {
    pub fn system_index(&self, node_index: NodeIndex<usize>) -> &SystemIndex {
        self.graph
            .node_weight(node_index)
            .expect("Could not get system index from node index!")
    }

    pub fn add_system(&mut self, system: SystemIndex) -> NodeIndex<usize> {
        self.graph.add_node(system)
    }

    pub fn add_system_dependency(
        &mut self,
        node: NodeIndex<usize>,
        node_indices: &HashMap<TypeId, NodeIndex<usize>>,
        dependency: &Dependency,
    ) -> Result<()> {
        let other_node = *node_indices
            .get(&dependency.boxed_system().system_type())
            .ok_or_else(|| {
                miette!(
                    "Failed to find dependency node index for {}",
                    dependency.boxed_system().system_name()
                )
            })?;
        dependency.boxed_system().system_name();

        match dependency {
            Dependency::Before(_) => {
                self.graph.add_edge(node, other_node, ());
            }
            Dependency::After(_) => {
                self.graph.add_edge(other_node, node, ());
            }
        }

        Ok(())
    }
}

/// Trait that allows systems to specify explicit system ordering.
pub trait IntoDependencySystem<Input>: Sized {
    fn into_dependency_system(self) -> DependencySystem<Input> {
        self.into_dependency_system_with_index(DEFAULT_ORDER_INDEX)
    }

    fn into_dependency_system_with_index(self, order_index: u8) -> DependencySystem<Input>;

    /// Schedule the system before the system supplied as argument.
    fn before<OtherInput>(
        self,
        system: impl IntoSystem<NormalSystem, OtherInput>,
    ) -> DependencySystem<()> {
        self.into_dependency_system().before(system)
    }

    /// Schedule the system after the system supplied as argument.
    fn after<OtherInput>(
        self,
        system: impl IntoSystem<NormalSystem, OtherInput>,
    ) -> DependencySystem<()> {
        self.into_dependency_system().after(system)
    }
}

pub enum Dependency {
    Before(BoxedSystem),
    After(BoxedSystem),
}

impl Dependency {
    fn boxed_system(&self) -> &BoxedSystem {
        match self {
            Dependency::Before(boxed_system) => boxed_system,
            Dependency::After(boxed_system) => boxed_system,
        }
    }
}

pub struct DependencySystem<I> {
    system: BoxedSystem,
    dependencies: Vec<Dependency>,
    system_order_index: u8,
    _input: PhantomData<I>,
}

impl DependencySystem<()> {
    pub(crate) fn boxed_system(&self) -> &BoxedSystem {
        &self.system
    }

    pub(crate) fn add_dependency(&mut self, dependency: Dependency) {
        self.dependencies.push(dependency)
    }

    pub(crate) fn system_order_index(&self) -> u8 {
        self.system_order_index
    }
}

// Get systems with all possible inputs
// This `I` gets replaced later as we do not need it
impl<S: IntoSystem<NormalSystem, I>, I> IntoDependencySystem<I> for S {
    fn into_dependency_system_with_index(self, order_index: u8) -> DependencySystem<I> {
        DependencySystem {
            system: Box::new(self.into_system()),
            dependencies: Vec::new(),
            system_order_index: order_index,
            _input: PhantomData,
        }
    }
}

impl<I> IntoDependencySystem<()> for DependencySystem<I> {
    fn into_dependency_system_with_index(self, order_index: u8) -> DependencySystem<()> {
        DependencySystem {
            system: self.system,
            dependencies: self.dependencies,
            system_order_index: order_index,
            _input: PhantomData,
        }
    }

    fn before<'a, Input>(
        self,
        system: impl IntoSystem<NormalSystem, Input>,
    ) -> DependencySystem<()> {
        let mut out = self.into_dependency_system();
        out.dependencies
            .push(Dependency::Before(Box::new(system.into_system())));
        out
    }

    fn after<'a, Input>(
        self,
        system: impl IntoSystem<NormalSystem, Input>,
    ) -> DependencySystem<()> {
        let mut out = self.into_dependency_system();
        out.dependencies
            .push(Dependency::After(Box::new(system.into_system())));
        out
    }
}

impl Hash for BoxedSystem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO: maybe should add dependency info???
        self.system_type().hash(state);
        self.system_name().hash(state);
    }
}

impl PartialEq for BoxedSystem {
    fn eq(&self, other: &Self) -> bool {
        self.system_type() == other.system_type()
    }
}

impl Eq for BoxedSystem {}

#[derive(Default)]
pub struct Schedule {
    dependency_systems: Vec<DependencySystem<()>>,
    dags: Vec<Dag>,
    node_index_lookup: Vec<HashMap<NodeIndex<usize>, usize>>,
}

impl Schedule {
    pub fn with_dependency_systems(dependency_systems: Vec<DependencySystem<()>>) -> Result<Self> {
        let mut schedule = Self::default();

        for ordering in dependency_systems {
            schedule.add_dependency_system(ordering)?;
        }

        Ok(schedule)
    }

    pub fn add_dependency_system(&mut self, dependency_system: DependencySystem<()>) -> Result<()> {
        self.dependency_systems.push(dependency_system);

        Ok(())
    }

    pub fn check_ordered_dependencies(&mut self) -> Result<()> {
        let dependency_system_lookup: HashMap<TypeId, &DependencySystem<()>> = self
            .dependency_systems
            .iter()
            .map(|dependency_system| (dependency_system.system.system_type(), dependency_system))
            .collect();

        for dependency_system in &self.dependency_systems {
            for dependency in &dependency_system.dependencies {
                let other_dependency_system = dependency_system_lookup
                    .get(&dependency.boxed_system().system_type())
                    .ok_or_else(|| {
                        miette!(
                            "Unable to find dependency \"{}\" for \"{}\"",
                            dependency.boxed_system().system_name(),
                            dependency.boxed_system().system_name()
                        )
                    })?;

                match dependency {
                    Dependency::Before(_) => {
                        if dependency_system.system_order_index()
                            > other_dependency_system.system_order_index()
                        {
                            return Err(miette!(
                                "{}.before({}) but {} has a higher order than {}, {} vs {}",
                                dependency_system.system.system_name(),
                                other_dependency_system.system.system_name(),
                                dependency_system.system.system_name(),
                                other_dependency_system.system.system_name(),
                                dependency_system.system_order_index(),
                                other_dependency_system.system_order_index(),
                            ));
                        }
                    }
                    Dependency::After(_) => {
                        if dependency_system.system_order_index()
                            < other_dependency_system.system_order_index()
                        {
                            return Err(miette!(
                                "{}.after({}) but {} has a lower order than {}, {} vs {}",
                                dependency_system.system.system_name(),
                                other_dependency_system.system.system_name(),
                                dependency_system.system.system_name(),
                                other_dependency_system.system.system_name(),
                                dependency_system.system_order_index(),
                                other_dependency_system.system_order_index(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn build_graph(&mut self) -> Result<()> {
        let dependency_system_lookup: HashMap<TypeId, &DependencySystem<()>> = self
            .dependency_systems
            .iter()
            .map(|dependency_system| (dependency_system.system.system_type(), dependency_system))
            .collect();

        let mut unique_system_order_indeces = self
            .dependency_systems
            .iter()
            .map(|dependency_system| dependency_system.system_order_index)
            .unique()
            .collect::<Vec<_>>();
        unique_system_order_indeces.sort_unstable();
        let mut node_indices_per_dag =
            vec![HashMap::<TypeId, NodeIndex<usize>>::new(); unique_system_order_indeces.len()];

        let mut system_order_to_dag_lookup = HashMap::new();

        for (dag_id, order_index) in unique_system_order_indeces.iter().enumerate() {
            self.dags.push(Dag::default());
            self.node_index_lookup.push(Default::default());
            system_order_to_dag_lookup.insert(*order_index, dag_id);
        }

        for (system_index, dependency_system) in self.dependency_systems.iter().enumerate() {
            let dag_index = system_order_to_dag_lookup[&dependency_system.system_order_index];
            let dag = &mut self.dags[dag_index];
            let node_indices = &mut node_indices_per_dag[dag_index];

            let node_index = dag.add_system(SystemIndex(system_index));
            node_indices.insert(dependency_system.system.system_type(), node_index);
            self.node_index_lookup[dag_index].insert(node_index, system_index);
        }

        for dependency_system in &self.dependency_systems {
            let dag_index = system_order_to_dag_lookup[&dependency_system.system_order_index];
            let dag = &mut self.dags[dag_index];
            let node_indices = &mut node_indices_per_dag[dag_index];
            let node_index = node_indices[&dependency_system.system.system_type()];

            for dependency in &dependency_system.dependencies {
                let dependency_dependency_system =
                    dependency_system_lookup[&dependency.boxed_system().system_type()];
                if dependency_system.system_order_index()
                    == dependency_dependency_system.system_order_index()
                {
                    dag.add_system_dependency(node_index, node_indices, dependency)?;
                }
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, storage: &mut Storage) -> Result<()> {
        for (dag_index, dag) in self.dags.iter().enumerate() {
            let mut execution_graph = dag.graph.clone();

            while execution_graph.node_count() > 0 {
                let current_nodes = match toposort(&execution_graph, None) {
                    // No cycle: get all nodes that have no dependencies
                    Ok(sorted) => sorted
                        .into_iter()
                        .take_while(|idx| {
                            execution_graph
                                .neighbors_directed(*idx, Direction::Incoming)
                                .count()
                                == 0
                        })
                        .collect::<HashSet<_>>(),
                    // Cycle: fix yo code
                    Err(cycle) => {
                        let dependency_system_index = *self.node_index_lookup[dag_index]
                            .get(&cycle.node_id())
                            .unwrap();
                        let dependency_system = &self.dependency_systems[dependency_system_index];

                        return Err(
                            miette! { "Cycle found in system {}", dependency_system.system.system_name()},
                        );
                    }
                };

                // TODO: parallel implementation
                for node in &current_nodes {
                    self.dependency_systems[dag.system_index(*node).0]
                        .system
                        .run(storage)?;
                    execution_graph.remove_node(*node);
                }
            }
        }

        Ok(())
    }
}
