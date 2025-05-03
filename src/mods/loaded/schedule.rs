use std::hash::{Hash, Hasher};

use bevy_platform::collections::{HashMap, HashSet};
use common::{StableId, Start, Update};
use petgraph::{algo::TarjanScc, prelude::*};

use super::LoadingError;
use crate::mods::{Cycle, SchedulingError};

type Dag<T> = DiGraphMap<T, ()>;

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
pub struct LoadedSchedules(HashMap<StableId, LoadedSchedule>);

impl LoadedSchedules {
    pub fn try_from_schedule_descriptors(
        descriptors: &Vec<common::ScheduleDescriptor>,
    ) -> Result<Self, LoadingError> {
        let mut schedules: HashMap<StableId, Vec<&common::Schedule>> = HashMap::default();

        // Allow only the default schedules for now
        schedules.insert(StableId::from_typed::<Start>(), Vec::new());
        schedules.insert(StableId::from_typed::<Update>(), Vec::new());

        // Group together schedules with the same schedule id
        for descriptor in descriptors {
            let schedule_id = descriptor.id.to_owned();
            schedules
                .get_mut(&schedule_id)
                .ok_or(LoadingError::InvalidSchedule(schedule_id))?
                .push(&descriptor.schedule);
        }

        let mut inner = HashMap::default();
        for (id, schedules) in schedules {
            if !schedules.is_empty() {
                let loaded = LoadedSchedule::try_from_schedules(&schedules[..])
                    .map_err(LoadingError::SchedulingError)?;
                inner.insert(id, loaded);
            }
        }

        Ok(Self(inner))
    }
}

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
pub struct LoadedSchedule {
    systems: HashMap<common::SystemId, LoadedSystem>,
    dependency: Dag<common::SystemId>,
}

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
struct LoadedSystem {
    /// Whether or not this system depends on any other system
    /// In the case this is false, the scheduler can run this system first
    is_dependent: bool,
    name: String,
    params: Vec<common::Param>,
}

impl LoadedSchedule {
    pub fn try_from_schedules(schedules: &[&common::Schedule]) -> Result<Self, SchedulingError> {
        let mut builder = Builder::default();

        // Add constraints to the dependency graph
        for schedule in schedules {
            for constraint in schedule.constraints.iter() {
                builder.add_constraint(constraint)?;
            }
        }

        let mut loaded_schedules = builder.build()?;

        // Add missing parameters to the systems
        for schedule in schedules {
            for common::System { id, name, params } in schedule.systems.iter() {
                let system = loaded_schedules.systems.entry(*id).or_insert(LoadedSystem {
                    is_dependent: false,
                    name: String::new(),
                    params: Vec::new(),
                });
                system.name = name.clone();
                system.params = params.iter().map(common::Param::to_owned).collect();
            }
        }

        Ok(loaded_schedules)
    }
}

#[derive(Default)]
struct Builder {
    dependency: Dag<Node>,
    sets: HashMap<SystemSet, usize>,
}

impl Builder {
    fn add_constraint(&mut self, constraint: &common::Constraint) -> Result<(), SchedulingError> {
        match constraint {
            common::Constraint::Order { before, after } => {
                let (_, before) = self.populate_set_nodes(before)?;
                let (after, _) = self.populate_set_nodes(after)?;
                self.dependency.add_edge(before, after, ());
            }
            common::Constraint::Condition { set, condition } => {
                let condition = Node::System(*condition);
                let (after, _) = self.populate_set_nodes(set)?;

                // The condition must run before the first node of the set
                self.dependency.add_edge(condition, after, ());
            }
            common::Constraint::Includes { parent_name, set } => {
                let parent = SystemSet::Named(parent_name.to_owned());
                let (start_parent, end_parent) = self.populate_set_nodes_inner(parent);
                let (start_set, end_set) = self.populate_set_nodes(set)?;

                // The child set must run within the parent set
                // So the first node of the child set must run after the first node of the parent set
                // And the last node of the child set must run before the last node of the parent set
                self.dependency.add_edge(start_parent, start_set, ());
                self.dependency.add_edge(end_set, end_parent, ());
            }
        }
        Ok(())
    }

    /// For a given set, resolves the before and after nodes
    fn populate_set_nodes(
        &mut self,
        set: &common::SystemSet,
    ) -> Result<(Node, Node), SchedulingError> {
        match set {
            common::SystemSet::Anonymous(systems) => match systems.len() {
                0 => Err(SchedulingError::EmptyAnonymousSet),
                1 => {
                    let id = Node::System(systems[0]);
                    Ok((id, id))
                }
                _ => {
                    let mut set = HashSet::new();
                    for system in systems {
                        set.insert(*system);
                    }
                    let set = SystemSet::Anonymous(set);
                    Ok(self.populate_set_nodes_inner(set))
                }
            },
            common::SystemSet::Named(name) => {
                let set = SystemSet::Named(name.to_owned());
                Ok(self.populate_set_nodes_inner(set))
            }
        }
    }

    fn populate_set_nodes_inner(&mut self, set: SystemSet) -> (Node, Node) {
        let id = self.sets.get(&set).map(|id| *id).unwrap_or_else(|| {
            let id = self.sets.len();

            // If this is an anonymous set, link its dependencies
            if let SystemSet::Anonymous(systems) = &set {
                for system in systems {
                    self.dependency
                        .add_edge(Node::SetStart(id), Node::System(*system), ());
                    self.dependency
                        .add_edge(Node::System(*system), Node::SetEnd(id), ());
                }
            }

            self.sets.insert(set, id);
            id
        });
        (Node::SetStart(id), Node::SetEnd(id))
    }

    fn build(self) -> Result<LoadedSchedule, SchedulingError> {
        let mut cycles = Vec::new();
        let mut reverse_nodes = Vec::with_capacity(self.dependency.node_count());
        TarjanScc::new().run(&self.dependency, |scc| {
            if scc.len() == 1 {
                reverse_nodes.push(scc[0]);
            } else {
                cycles.push(Cycle(
                    scc.iter()
                        .filter_map(|node| match node {
                            Node::System(system) => Some(*system),
                            _ => None,
                        })
                        .collect(),
                ));
            }
        });

        if !cycles.is_empty() {
            return Err(SchedulingError::Cycles {
                named_set: None,
                cycles,
            });
        }

        let mut systems = HashMap::new();
        let mut dependency = Dag::new();
        let mut is_dependent = false;
        for id in reverse_nodes
            .into_iter()
            .rev()
            .filter_map(|node| match node {
                Node::System(system) => Some(system),
                _ => None,
            })
        {
            is_dependent |= self
                .dependency
                .neighbors_directed(Node::System(id), Direction::Incoming)
                .next()
                .is_some();
            systems.insert(
                id,
                LoadedSystem {
                    is_dependent,
                    name: String::new(),
                    params: Vec::new(),
                },
            );
            self.add_node_dependents_to_flattened(&mut dependency, id, Node::System(id));
        }

        Ok(LoadedSchedule {
            systems,
            dependency,
        })
    }

    fn add_node_dependents_to_flattened(
        &self,
        dependency: &mut Dag<common::SystemId>,
        parent: common::SystemId,
        node: Node,
    ) {
        for child in self
            .dependency
            .neighbors_directed(node, Direction::Outgoing)
        {
            match child {
                Node::System(system) => {
                    dependency.add_edge(parent, system, ());
                }
                _ => self.add_node_dependents_to_flattened(dependency, parent, child),
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum SystemSet {
    Anonymous(HashSet<common::SystemId>),
    Named(common::StableId),
}

impl Hash for SystemSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Anonymous(systems) => {
                // Note: order will remain the same regardless of id insertion order
                for system in systems {
                    system.hash(state);
                }
            }
            Self::Named(id) => id.hash(state),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
enum Node {
    System(common::SystemId),
    SetStart(usize),
    SetEnd(usize),
}
