use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{
    storage::{BoxedSystem, Storage},
    system::IntoSystem,
};

use color_eyre::{eyre::eyre, Result};
use petgraph::{algo::toposort, prelude::NodeIndex, stable_graph::StableDiGraph, Direction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemIndex(usize);

#[derive(Default)]
pub struct Dag {
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

// We need the input generic param for it to compile but its not used so we force it to ()
pub trait IntoSystemOrdering<Input>: Sized {
    fn into_system_ordering(self) -> SystemOrdering<Input>;
    fn before<OtherInput>(self, system: impl IntoSystem<OtherInput>) -> SystemOrdering<()> {
        self.into_system_ordering().before(system)
    }
    fn after<OtherInput>(self, system: impl IntoSystem<OtherInput>) -> SystemOrdering<()> {
        self.into_system_ordering().after(system)
    }
}

pub enum Dependency {
    Before(BoxedSystem),
    After(BoxedSystem),
}

pub struct SystemOrdering<I> {
    system: BoxedSystem,
    dependencies: Vec<Dependency>,
    _input: PhantomData<I>,
}

// Get systems with all possible inputs
// This `I` gets replaced later as we do not need it
impl<S: IntoSystem<I>, I> IntoSystemOrdering<I> for S {
    fn into_system_ordering(self) -> SystemOrdering<I> {
        SystemOrdering {
            system: Box::new(self.into_system()),
            dependencies: Vec::new(),
            _input: PhantomData,
        }
    }
}

impl IntoSystemOrdering<()> for BoxedSystem {
    fn into_system_ordering(self) -> SystemOrdering<()> {
        SystemOrdering {
            system: self,
            dependencies: Vec::new(),
            _input: PhantomData,
        }
    }
}

impl<I> IntoSystemOrdering<()> for SystemOrdering<I> {
    fn into_system_ordering(self) -> SystemOrdering<()> {
        SystemOrdering {
            system: self.system,
            dependencies: self.dependencies,
            _input: PhantomData,
        }
    }

    fn before<'a, Input>(self, system: impl IntoSystem<Input>) -> SystemOrdering<()> {
        let mut out = self.into_system_ordering();
        out.dependencies
            .push(Dependency::Before(Box::new(system.into_system())));
        out
    }

    fn after<'a, Input>(self, system: impl IntoSystem<Input>) -> SystemOrdering<()> {
        let mut out = self.into_system_ordering();
        out.dependencies
            .push(Dependency::After(Box::new(system.into_system())));
        out
    }
}

impl Schedule {
    pub fn with_system_orderings(system_orderings: Vec<SystemOrdering<()>>) -> Result<Self> {
        let mut schedule = Self::default();

        for ordering in system_orderings {
            schedule.add_system_ordering(ordering)?;
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
    pub fn add_system_ordering(&mut self, ordering: SystemOrdering<()>) -> Result<()> {
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
                    return Err(eyre! { "Cycle found at node `{:?}`",
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
