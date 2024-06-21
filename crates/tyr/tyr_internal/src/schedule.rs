use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    process::exit,
};

use itertools::Itertools;

use crate::{
    storage::{BoxedSystem, Storage},
    system::{IntoSystem, NormalSystem},
};

use miette::{miette, Result};
use petgraph::{
    algo::toposort, csr::IndexType, prelude::NodeIndex, stable_graph::StableDiGraph, Direction,
};

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

    // pub fn add_system(
    //     &mut self,
    //     node_indices: &HashMap<BoxedSystem, NodeIndex<usize>>,
    //     system: SystemIndex,
    //     dependencies: &[Dependency],
    // ) -> Result<NodeIndex<usize>> {
    //     let node = self.graph.add_node(system);
    //     for dependency in dependencies {
    //         match dependency {
    //             Dependency::Before(other) => {
    //                 let other_node = *node_indices
    //                     .get(other)
    //                     .expect("Failed to find dependency node index"); // TODO: nice error handling
    //                 self.graph.add_edge(node, other_node, ());
    //             }
    //             Dependency::After(other) => {
    //                 let other_node = *node_indices
    //                     .get(other)
    //                     .expect("Failed to find dependency node index"); // TODO: nice error handling
    //                 self.graph.add_edge(other_node, node, ());
    //             }
    //         }
    //     }
    //
    //     Ok(node)
    // }
    pub fn add_system(
        &mut self,
        node_indices: &HashMap<TypeId, NodeIndex<usize>>,
        system: SystemIndex,
        dependencies: &[Dependency],
    ) -> Result<NodeIndex<usize>> {
        let node = self.graph.add_node(system);
        for dependency in dependencies {
            match dependency {
                Dependency::Before(other) => {
                    let other_node = *node_indices
                        .get(&other.deref().system_type())
                        .expect("Failed to find dependency node index"); // TODO: nice error handling
                    self.graph.add_edge(node, other_node, ());
                }
                Dependency::After(other) => {
                    let other_node = *node_indices
                        .get(&other.deref().system_type())
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
    // systems: Vec<BoxedSystem>,
    dependency_systems: Vec<DependencySystem<()>>,
    // node_indices: HashMap<BoxedSystem, NodeIndex<usize>>,
    dag: Dag,
    dags: Vec<Dag>,
}

/// Trait that allows systems to specify explicit system ordering.
pub trait IntoDependencySystem<Input>: Sized {
    fn into_dependency_system(self) -> DependencySystem<Input> {
        self.into_dependency_system_with_order_index(50)
    }

    fn into_dependency_system_with_order_index(self, order_index: u8) -> DependencySystem<Input>;

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
    fn into_dependency_system(self) -> DependencySystem<I> {
        DependencySystem {
            system: Box::new(self.into_system()),
            dependencies: Vec::new(),
            system_order_index: 50,
            _input: PhantomData,
        }
    }

    fn into_dependency_system_with_order_index(self, order_index: u8) -> DependencySystem<I> {
        DependencySystem {
            system: Box::new(self.into_system()),
            dependencies: Vec::new(),
            system_order_index: order_index,
            _input: PhantomData,
        }
    }
}

impl<I> IntoDependencySystem<()> for DependencySystem<I> {
    fn into_dependency_system_with_order_index(self, order_index: u8) -> DependencySystem<()> {
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
        self.dependency_systems.push(ordering);

        Ok(())
    }

    pub fn check_ordered_dependencies(&mut self) -> Result<()> {
        let boxed_system_lookup = HashMap::<TypeId, &DependencySystem<()>>::from_iter(
            self.dependency_systems.iter().map(|dependency_system| {
                (
                    dependency_system.system.deref().system_type(),
                    dependency_system,
                )
            }),
        );

        for dependency_system in &self.dependency_systems {
            for dependency in &dependency_system.dependencies {
                match dependency {
                    Dependency::Before(other) => {
                        let other_boxed_dependency = boxed_system_lookup
                            .get(&other.deref().system_type())
                            .unwrap();

                        if dependency_system.system_order_index()
                            > other_boxed_dependency.system_order_index()
                        {
                            eprintln!(
                                "{} should be before {}",
                                dependency_system.system.system_name(),
                                other_boxed_dependency.system.system_name()
                            );
                            exit(1);
                        }
                    }
                    Dependency::After(other) => {
                        let other_boxed_dependency = boxed_system_lookup
                            .get(&other.deref().system_type())
                            .unwrap();

                        if dependency_system.system_order_index()
                            < other_boxed_dependency.system_order_index()
                        {
                            eprintln!(
                                "{} should be after {}",
                                dependency_system.system.system_name(),
                                other_boxed_dependency.system.system_name()
                            );
                            exit(1);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn build_graph(&mut self) -> Result<()> {
        let mut unique_system_order_indeces = self
            .dependency_systems
            .iter()
            .map(|dependency_system| dependency_system.system_order_index)
            .unique()
            .collect::<Vec<_>>();
        unique_system_order_indeces.sort_unstable();

        let mut system_order_to_dag_lookup = HashMap::new();

        for (dag_id, order_index) in unique_system_order_indeces.iter().enumerate() {
            self.dags.push(Dag::default());
            system_order_to_dag_lookup.insert(*order_index, dag_id);
        }

        let mut node_indices = HashMap::new();
        for (system_index, dependency_system) in self.dependency_systems.iter().enumerate() {
            let dag =
                &mut self.dags[system_order_to_dag_lookup[&dependency_system.system_order_index]];
            eprintln!(
                "system_name: {:?}",
                dependency_system.system.deref().system_name()
            );
            let node_index = dag.add_system(
                &node_indices,
                SystemIndex(system_index),
                &dependency_system.dependencies,
            )?;
            eprintln!("node_id: {}", node_index.index());
            node_indices.insert(dependency_system.system.deref().system_type(), node_index);
        }

        Ok(())
    }

    // fn system(&self, node_id: NodeIndex<usize>) -> &BoxedSystem {
    //     // &self.systems[self.dag.system_index(node_id).0]
    //     &self.dependency_systems[self.dag.system_index(node_id).0].system
    // }
    //
    // fn system_name(&self, node_id: NodeIndex<usize>) -> &str {
    //     self.system(node_id).system_name()
    // }

    // #[allow(dead_code)]
    // fn print_graph(&mut self) {
    //     let nodes = self.dag.graph.node_indices();
    //     for idx in nodes {
    //         let name = self.dependency_systems[self.dag.graph.node_weight(idx).unwrap().0]
    //             .system
    //             .system_name();
    //         let incoming = self
    //             .dag
    //             .graph
    //             .neighbors_directed(idx, Direction::Incoming)
    //             .map(|neighbor| self.system_name(neighbor))
    //             .collect::<Vec<_>>();
    //
    //         let outgoing = self
    //             .dag
    //             .graph
    //             .neighbors_directed(idx, Direction::Outgoing)
    //             .map(|neighbor| self.system_name(neighbor))
    //             .collect::<Vec<_>>();
    //
    //         println!("System `{name}`\nIncoming: `{incoming:?}`\nOutgoing: `{outgoing:?}`\n");
    //     }
    // }

    pub fn execute(&mut self, storage: &mut Storage) -> Result<()> {
        for dag in &self.dags {
            // let mut execution_graph = self.dag.graph.clone();
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
                    Err(_cycle) => {
                        return Err(miette! { "Cycle found"});
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
