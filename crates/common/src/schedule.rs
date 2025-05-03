use super::*;

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ScheduleDescriptor {
    pub id: StableId,
    pub schedule: Schedule,
}

/// Describes how to create a schedule
#[derive(Encode, Decode, PartialEq, Debug, Default, Clone)]
pub struct Schedule {
    pub systems: Vec<System>,
    pub constraints: Vec<Constraint>,
}

/// Constraints that define the order of systems in the schedule
///
/// These must always be checked for validity before being loaded by the modloader
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum Constraint {
    /// One system set needs to run before another system set
    Order { before: SystemSet, after: SystemSet },
    /// System set needs to run only if the condition is met
    Condition { set: SystemSet, condition: SystemId },
    /// A system set is included in a named set
    Includes {
        parent_name: StableId,
        set: SystemSet,
    },
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct System {
    pub id: SystemId,
    pub name: String,
    pub params: Vec<Param>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum SystemSet {
    Anonymous(Vec<SystemId>),
    Named(StableId),
}
