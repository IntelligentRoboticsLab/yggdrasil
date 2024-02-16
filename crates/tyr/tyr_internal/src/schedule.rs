use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{
    storage::{BoxedSystem, Storage},
    system::{IntoSystem, NormalSystem},
};

use miette::{miette, Result};
use petgraph::{algo::toposort, prelude::NodeIndex, stable_graph::StableDiGraph, Direction};

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

    pub fn add_system(
        &mut self,
        node_indices: &HashMap<BoxedSystem, NodeIndex<usize>>,
        system: SystemIndex,
        dependencies: &[Dependency],
    ) -> Result<NodeIndex<usize>> {
        let node = self.graph.add_node(system);
        for dependency in dependencies {
            match dependency {
                Dependency::Before(other) => {
                    let other_node = *node_indices
                        .get(other)
                        .expect("Failed to find dependency node index"); // TODO: nice error handling
                    self.graph.add_edge(node, other_node, ());
                }
                Dependency::After(other) => {
                    let other_node = *node_indices
                        .get(other)
                        .expect("Failed to find dependency node index"); // TODO: nice error handling
                    self.graph.add_edge(other_node, node, ());
                }
            }
        }

        Ok(node)
    }
}

#[derive(Default)]
pub struct Schedule {
    systems: Vec<BoxedSystem>,
    node_indices: HashMap<BoxedSystem, NodeIndex<usize>>,
    dag: Dag,
}

/// Trait that allows systems to specify explicit system ordering.
pub trait IntoDependencySystem<Input>: Sized {
    fn into_dependency_system(self) -> DependencySystem<Input>;

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

pub struct DependencySystem<I> {
    system: BoxedSystem,
    dependencies: Vec<Dependency>,
    _input: PhantomData<I>,
}

impl DependencySystem<()> {
    pub(crate) fn boxed_system(&self) -> &BoxedSystem {
        &self.system
    }

    pub(crate) fn add_dependency(&mut self, dependency: Dependency) {
        self.dependencies.push(dependency)
    }
}

// Get systems with all possible inputs
// This `I` gets replaced later as we do not need it
impl<S: IntoSystem<NormalSystem, I>, I> IntoDependencySystem<I> for S {
    fn into_dependency_system(self) -> DependencySystem<I> {
        DependencySystem {
            system: Box::new(self.into_system()),
            dependencies: Vec::new(),
            _input: PhantomData,
        }
    }
}

impl<I> IntoDependencySystem<()> for DependencySystem<I> {
    fn into_dependency_system(self) -> DependencySystem<()> {
        DependencySystem {
            system: self.system,
            dependencies: self.dependencies,
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

impl Schedule {
    pub fn with_dependency_systems(dependency_systems: Vec<DependencySystem<()>>) -> Result<Self> {
        let mut schedule = Self::default();

        for ordering in dependency_systems {
            schedule.add_dependency_system(ordering)?;
        }

        Ok(schedule)
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

impl Schedule {
    pub fn add_dependency_system(&mut self, ordering: DependencySystem<()>) -> Result<()> {
        let system_index = SystemIndex(self.systems.len());
        self.systems.push(ordering.system.clone());

        let node_index =
            self.dag
                .add_system(&self.node_indices, system_index, &ordering.dependencies)?;
        self.node_indices.insert(ordering.system, node_index);

        Ok(())
    }

    fn system(&self, node_id: NodeIndex<usize>) -> &BoxedSystem {
        &self.systems[self.dag.system_index(node_id).0]
    }

    fn system_name(&self, node_id: NodeIndex<usize>) -> &str {
        self.system(node_id).system_name()
    }

    #[allow(dead_code)]
    fn print_graph(&mut self) {
        let nodes = self.dag.graph.node_indices();
        for idx in nodes {
            let name = self.systems[self.dag.graph.node_weight(idx).unwrap().0].system_name();
            let incoming = self
                .dag
                .graph
                .neighbors_directed(idx, Direction::Incoming)
                .map(|neighbor| self.system_name(neighbor))
                .collect::<Vec<_>>();

            let outgoing = self
                .dag
                .graph
                .neighbors_directed(idx, Direction::Outgoing)
                .map(|neighbor| self.system_name(neighbor))
                .collect::<Vec<_>>();

            println!("System `{name}`\nIncoming: `{incoming:?}`\nOutgoing: `{outgoing:?}`\n");
        }
    }

    pub fn execute(&mut self, storage: &mut Storage) -> Result<()> {
        let mut execution_graph = self.dag.graph.clone();

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
                    return Err(miette! { "Cycle found at node `{:?}`",
                    self.system_name(cycle.node_id()) })
                }
            };

            // TODO: parallel implementation
            for node in &current_nodes {
                self.systems[self.dag.system_index(*node).0].run(storage)?;
                execution_graph.remove_node(*node);
            }
        }

        Ok(())
    }
}
