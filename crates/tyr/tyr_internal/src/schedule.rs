use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
    marker::PhantomData,
    path::Path,
};

use itertools::Itertools;
use tracing::{info_span, Level};

use crate::{
    storage::{BoxedSystem, Storage},
    system::{IntoSystem, NormalSystem},
};

use miette::{miette, IntoDiagnostic, Report, Result};
use petgraph::{algo::toposort, prelude::NodeIndex, stable_graph::StableDiGraph, Direction};

const DEFAULT_ORDER_INDEX: u8 = (256u32 / 2u32) as u8;
use graphviz_rust::{
    cmd::Format,
    printer::{DotPrinter, PrinterContext},
};

use dot_structures::{Attribute, Edge, EdgeTy, Graph, Id, Node, NodeId, Stmt, Vertex};

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

    fn current_index(&self) -> u8;

    /// Schedule the system before the system supplied as argument.
    fn before<OtherInput>(
        self,
        system: impl IntoSystem<NormalSystem, OtherInput>,
    ) -> DependencySystem<()> {
        let current_index = self.current_index();
        self.into_dependency_system_with_index(current_index)
            .before(system)
    }

    /// Schedule the system after the system supplied as argument.
    fn after<OtherInput>(
        self,
        system: impl IntoSystem<NormalSystem, OtherInput>,
    ) -> DependencySystem<()> {
        let current_index = self.current_index();
        self.into_dependency_system_with_index(current_index)
            .after(system)
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct DependencySystem<I> {
    system: BoxedSystem,
    enabled: bool,
    dependencies: Vec<Dependency>,
    system_order_index: u8,
    _input: PhantomData<I>,
}

impl DependencySystem<()> {
    pub(crate) fn system_name(&self) -> &str {
        self.system.system_name()
    }

    pub(crate) fn run(&mut self, storage: &mut Storage) -> Result<()> {
        if self.enabled {
            self.system.run(storage)
        } else {
            Ok(())
        }
    }

    pub(crate) fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

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
            enabled: true,
            dependencies: Vec::new(),
            system_order_index: order_index,
            _input: PhantomData,
        }
    }

    fn current_index(&self) -> u8 {
        DEFAULT_ORDER_INDEX
    }
}

impl<I> IntoDependencySystem<()> for DependencySystem<I> {
    fn into_dependency_system_with_index(self, order_index: u8) -> DependencySystem<()> {
        DependencySystem {
            system: self.system,
            enabled: true,
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

    fn current_index(&self) -> u8 {
        self.system_order_index
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
        fn dependency_before_error(
            dependency_system: &DependencySystem<()>,
            other_dependency_system: &DependencySystem<()>,
        ) -> Report {
            miette!(
                "{}.before({}) but {} has a higher order than {}, {} vs {}",
                dependency_system.system.system_name(),
                other_dependency_system.system.system_name(),
                dependency_system.system.system_name(),
                other_dependency_system.system.system_name(),
                dependency_system.system_order_index(),
                other_dependency_system.system_order_index(),
            )
        }

        fn dependency_after_error(
            dependency_system: &DependencySystem<()>,
            other_dependency_system: &DependencySystem<()>,
        ) -> Report {
            miette!(
                "{}.after({}) but {} has a lower order than {}, {} vs {}",
                dependency_system.system.system_name(),
                other_dependency_system.system.system_name(),
                dependency_system.system.system_name(),
                other_dependency_system.system.system_name(),
                dependency_system.system_order_index(),
                other_dependency_system.system_order_index(),
            )
        }

        fn assert_all_dependencies_valid(
            dependency_system: &DependencySystem<()>,
            dependency_system_lookup: &HashMap<TypeId, &DependencySystem<()>>,
        ) -> Result<()> {
            for dependency in &dependency_system.dependencies {
                let other_dependency_system = dependency_system_lookup
                    .get(&dependency.boxed_system().system_type())
                    .ok_or_else(|| {
                        miette!(
                            "Unable to find dependency `{}` for `{}`",
                            dependency.boxed_system().system_name(),
                            dependency_system.boxed_system().system_name()
                        )
                    })?;

                match dependency {
                    Dependency::Before(_) => {
                        if dependency_system.system_order_index()
                            > other_dependency_system.system_order_index()
                        {
                            return Err(dependency_before_error(
                                dependency_system,
                                other_dependency_system,
                            ));
                        }
                    }
                    Dependency::After(_) => {
                        if dependency_system.system_order_index()
                            < other_dependency_system.system_order_index()
                        {
                            return Err(dependency_after_error(
                                dependency_system,
                                other_dependency_system,
                            ));
                        }
                    }
                }
            }

            Ok(())
        }

        let dependency_system_lookup: HashMap<TypeId, &DependencySystem<()>> = self
            .dependency_systems
            .iter()
            .map(|dependency_system| (dependency_system.system.system_type(), dependency_system))
            .collect();

        for dependency_system in &self.dependency_systems {
            assert_all_dependencies_valid(dependency_system, &dependency_system_lookup)?;
        }

        Ok(())
    }

    pub fn build_graph(&mut self) -> Result<()> {
        let dependency_system_lookup: HashMap<TypeId, &DependencySystem<()>> = self
            .dependency_systems
            .iter()
            .map(|dependency_system| (dependency_system.system.system_type(), dependency_system))
            .collect();

        let mut unique_system_order_indices = self
            .dependency_systems
            .iter()
            .map(|dependency_system| dependency_system.system_order_index)
            .unique()
            .collect::<Vec<_>>();
        unique_system_order_indices.sort_unstable();
        let mut node_indices_per_dag =
            vec![HashMap::<TypeId, NodeIndex<usize>>::new(); unique_system_order_indices.len()];

        let mut system_order_to_dag_lookup = HashMap::new();
        for (dag_id, order_index) in unique_system_order_indices.iter().enumerate() {
            self.dags.push(Dag::default());
            self.node_index_lookup.push(Default::default());
            system_order_to_dag_lookup.insert(*order_index, dag_id);
        }

        // First store all the systems before checking all the dependencies.
        // We do not check the dependencies while storing the systems, because we could depend on a
        // system that hasn't been stored yet.
        for (system_index, dependency_system) in self.dependency_systems.iter().enumerate() {
            let dag_index = system_order_to_dag_lookup[&dependency_system.system_order_index];
            let dag = &mut self.dags[dag_index];
            let node_indices = &mut node_indices_per_dag[dag_index];

            let node_index = dag.add_system(SystemIndex(system_index));
            node_indices.insert(dependency_system.system.system_type(), node_index);
            self.node_index_lookup[dag_index].insert(node_index, system_index);
        }

        // Now check all the dependencies.
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
        let span = tracing::span!(Level::TRACE, "execute");
        let _enter = span.enter();

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
                    let span = info_span!(
                        "system",
                        name = self.dependency_systems[dag.system_index(*node).0].system_name()
                    );
                    let _enter = span.enter();
                    self.dependency_systems[dag.system_index(*node).0].run(storage)?;
                    execution_graph.remove_node(*node);
                }
            }
        }

        Ok(())
    }

    pub fn generate_dot_file(&self) -> Result<String> {
        fn node(boxed_system: &BoxedSystem) -> Stmt {
            let mut hasher = std::hash::DefaultHasher::new();
            boxed_system.system_type().hash(&mut hasher);

            let attributes = vec![Attribute(
                Id::Plain("label".to_owned()),
                Id::Plain(format!("\"{}\"", boxed_system.system_name())),
            )];
            Stmt::Node(Node {
                id: NodeId(Id::Escaped(hasher.finish().to_string()), None),
                attributes,
            })
        }

        fn edge(boxed_system1: &BoxedSystem, boxed_system2: &BoxedSystem) -> Stmt {
            let mut hasher1 = std::hash::DefaultHasher::new();
            boxed_system1.system_type().hash(&mut hasher1);
            let mut hasher2 = std::hash::DefaultHasher::new();
            boxed_system2.system_type().hash(&mut hasher2);

            Stmt::Edge(Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(hasher1.finish().to_string()), None)),
                    Vertex::N(NodeId(Id::Plain(hasher2.finish().to_string()), None)),
                ),
                attributes: vec![],
            })
        }

        let mut statements: Vec<Stmt> = Vec::new();

        for dependency_system in &self.dependency_systems {
            statements.push(node(&dependency_system.system))
        }

        for dependency_system in &self.dependency_systems {
            for dependency in &dependency_system.dependencies {
                statements.push(edge(&dependency_system.system, dependency.boxed_system()));
            }
        }

        let graph = Graph::DiGraph {
            id: Id::Plain("0".to_owned()),
            strict: true,
            stmts: statements,
        };

        Ok(graph.print(&mut PrinterContext::default()))
    }

    pub fn generate_graph<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let dot = self.generate_dot_file()?;

        let png =
            graphviz_rust::exec_dot(dot.clone(), vec![Format::Png.into()]).into_diagnostic()?;

        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(&png).into_diagnostic()?;

        Ok(())
    }

    pub fn get_system_by_name(&mut self, name: &str) -> Option<&mut DependencySystem<()>> {
        self.dependency_systems
            .iter_mut()
            .find(|sys| sys.system_name() == name)
    }

    pub fn get_system_by_index(&mut self, index: usize) -> Option<&mut DependencySystem<()>> {
        self.dependency_systems.get_mut(index)
    }

    pub fn list_systems(&self) -> HashMap<String, bool> {
        self.dependency_systems
            .iter()
            .map(|sys| (sys.system_name().to_string(), sys.enabled))
            .collect()
    }
}
