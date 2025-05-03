use crate::ecs::Reflected;

use super::{
    system_set::{SystemSet, Systems},
    IntoSystemSet,
};
use common::StableId;
use const_vec::ConstVec;

extern crate alloc;
use alloc::vec::Vec;

/// Similar in role to bevy's IntoSystemConfigs trait
#[const_trait]
pub trait IntoSchedule<M>
where
    Self: Sized,
{
    fn into_schedule(self) -> Schedule;

    fn chain(self) -> Schedule {
        let mut schedule = self.into_schedule();
        schedule.constraints.push(Constraint::Chain);
        schedule
    }

    fn before<Marker>(self, system_set: impl IntoSystemSet<Marker>) -> Schedule {
        let mut schedule = self.into_schedule();
        schedule
            .constraints
            .push(Constraint::Before(system_set_getter(system_set)));
        schedule
    }

    fn after<Marker>(self, system_set: impl IntoSystemSet<Marker>) -> Schedule {
        let mut schedule = self.into_schedule();
        schedule
            .constraints
            .push(Constraint::After(system_set_getter(system_set)));
        schedule
    }

    fn in_set(self, named_system_set: impl Reflected) -> Schedule {
        let mut schedule = self.into_schedule();
        schedule
            .constraints
            .push(Constraint::Includes(stable_id_getter(named_system_set)));
        schedule
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    system_set_getter: fn() -> SystemSet,
    systems_getter: fn() -> Systems,
    constraints: ConstVec<Constraint, 16>,
}

impl const IntoSchedule<()> for Schedule {
    fn into_schedule(self) -> Schedule {
        self
    }
}

impl Schedule {
    pub(crate) fn build(self) -> common::Schedule {
        let mut constraints = Vec::new();

        for constraint in self.constraints.into_slice() {
            match constraint {
                Constraint::Chain => {
                    let sets = (self.system_set_getter)().into_max_sets();
                    for set in sets.windows(2) {
                        constraints.push(common::Constraint::Order {
                            before: set[0].clone(),
                            after: set[1].clone(),
                        });
                    }
                }
                Constraint::Before(after_getter) => {
                    let before = (self.system_set_getter)().into_min_sets();
                    let after = (after_getter)().into_min_sets();
                    for before in before.iter() {
                        for after in after.iter() {
                            constraints.push(common::Constraint::Order {
                                before: before.clone(),
                                after: after.clone(),
                            });
                        }
                    }
                }
                Constraint::After(before_getter) => {
                    let before = (before_getter)().into_min_sets();
                    let after = (self.system_set_getter)().into_min_sets();
                    for before in before.iter() {
                        for after in after.iter() {
                            constraints.push(common::Constraint::Order {
                                before: before.clone(),
                                after: after.clone(),
                            });
                        }
                    }
                }
                Constraint::Includes(parent_name_getter) => {
                    let sets = (self.system_set_getter)().into_min_sets();
                    for set in sets {
                        constraints.push(common::Constraint::Includes {
                            parent_name: parent_name_getter(),
                            set,
                        });
                    }
                }
            }
        }

        common::Schedule {
            systems: (self.systems_getter)().0,
            constraints,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Constraint {
    Chain,
    Before(fn() -> SystemSet),
    After(fn() -> SystemSet),
    Includes(fn() -> StableId),
}

#[inline]
const fn system_set_getter<T, Marker>(_systems: T) -> fn() -> SystemSet
where
    T: IntoSystemSet<Marker>,
{
    T::into_system_set
}

#[inline]
const fn stable_id_getter<T>(_typed: T) -> fn() -> StableId
where
    T: Reflected,
{
    StableId::from_typed::<T>
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct FunctionMarker;

impl<Marker, F> const IntoSchedule<(FunctionMarker, Marker)> for F
where
    F: IntoSystemSet<Marker>,
{
    fn into_schedule(self) -> Schedule {
        Schedule {
            system_set_getter: F::into_system_set,
            systems_getter: F::into_systems,
            constraints: ConstVec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use bevy_reflect::Reflect;
    use common::{Param, System};

    extern crate alloc;
    use alloc::borrow::ToOwned;

    use super::*;
    use crate::{ecs::system::IntoSystem, prelude::Commands};

    fn get_anonymous_system_set<Marker>(
        system: impl IntoSystem<(), (), Marker>,
    ) -> common::SystemSet {
        common::SystemSet::Anonymous(vec![system.get_system_id()])
    }

    fn get_named_system_set<T>() -> common::SystemSet
    where
        T: Reflected,
    {
        common::SystemSet::Named(StableId::from_typed::<T>())
    }

    fn make_system<Marker, F>(system: F, params: Vec<Param>) -> System
    where
        F: IntoSystem<(), (), Marker>,
    {
        System {
            id: system.get_system_id(),
            name: system.get_name().to_owned(),
            params,
        }
    }

    #[test]
    fn into_config() {
        fn system1() {}
        fn system2(mut _commands: Commands) {}

        const SCHEDULE: Schedule = (system1, system2).into_schedule();

        let schedule = SCHEDULE.build();
        assert_eq!(
            schedule,
            common::Schedule {
                systems: vec![
                    make_system(system1, vec![]),
                    make_system(system2, vec![Param::Command]),
                ],
                constraints: vec![]
            }
        );
    }

    #[test]
    fn chaining() {
        fn system1() {}
        fn system2() {}
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet;

        const SCHEDULE: Schedule = (system1, system2, NamedSet).chain();

        assert_eq!(
            SCHEDULE.build().constraints,
            vec![
                common::Constraint::Order {
                    before: get_anonymous_system_set(system1),
                    after: get_anonymous_system_set(system2),
                },
                common::Constraint::Order {
                    before: get_anonymous_system_set(system2),
                    after: get_named_system_set::<NamedSet>(),
                }
            ]
        );
    }

    #[test]
    fn before_and_after() {
        fn system1() {}
        fn system2() {}
        fn system3() {}
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet1;
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet2;

        const SCHEDULE: Schedule = (system1, NamedSet1)
            .after(system2)
            .before((NamedSet2, system3));

        assert_eq!(
            SCHEDULE.build().constraints,
            vec![
                common::Constraint::Order {
                    before: get_anonymous_system_set(system2),
                    after: get_named_system_set::<NamedSet1>(),
                },
                common::Constraint::Order {
                    before: get_anonymous_system_set(system2),
                    after: get_anonymous_system_set(system1),
                },
                common::Constraint::Order {
                    before: get_named_system_set::<NamedSet1>(),
                    after: get_named_system_set::<NamedSet2>(),
                },
                common::Constraint::Order {
                    before: get_named_system_set::<NamedSet1>(),
                    after: get_anonymous_system_set(system3),
                },
                common::Constraint::Order {
                    before: get_anonymous_system_set(system1),
                    after: get_named_system_set::<NamedSet2>(),
                },
                common::Constraint::Order {
                    before: get_anonymous_system_set(system1),
                    after: get_anonymous_system_set(system3),
                }
            ]
        );
    }

    #[test]
    fn in_set() {
        fn system() {}
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet;

        const SCHEDULE: Schedule = system.in_set(NamedSet);

        assert_eq!(
            SCHEDULE.build().constraints,
            vec![common::Constraint::Includes {
                parent_name: StableId::from_typed::<NamedSet>(),
                set: get_anonymous_system_set(system),
            },]
        );
    }
}
