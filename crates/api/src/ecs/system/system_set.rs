use super::IntoSystem;
use crate::ecs::Reflected;
use common::{StableId, System, SystemId};
use variadics_please::all_tuples;

extern crate alloc;
use alloc::{vec, vec::Vec};

/// Similar in role to bevy's IntoSystemConfigs trait
pub trait IntoSystemSet<Marker>
where
    Self: Copy,
{
    fn into_system_set() -> SystemSet;

    fn into_systems() -> Systems {
        Systems(Vec::new())
    }
}

pub struct SystemSet(Vec<Sys>);

pub struct Systems(pub(crate) Vec<System>);

#[derive(PartialEq, Debug)]
enum Sys {
    Anonymous(SystemId),
    Named(StableId),
}

impl SystemSet {
    /// Returns a list of system sets. All anonymous systems will be contained in a single set.
    pub(crate) fn into_min_sets(self) -> Vec<common::SystemSet> {
        let mut anonymous = Vec::new();
        let mut sets = Vec::new();

        for sys in self.0 {
            match sys {
                Sys::Anonymous(id) => anonymous.push(id),
                Sys::Named(name) => sets.push(common::SystemSet::Named(name)),
            }
        }

        if !anonymous.is_empty() {
            sets.push(common::SystemSet::Anonymous(anonymous));
        }

        sets
    }

    /// Returns a list of system sets. Each anonymous system will have its own set
    pub(crate) fn into_max_sets(self) -> Vec<common::SystemSet> {
        self.0
            .into_iter()
            .map(|sys| match sys {
                Sys::Anonymous(id) => common::SystemSet::Anonymous(vec![id]),
                Sys::Named(name) => common::SystemSet::Named(name),
            })
            .collect()
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SystemMarker;

// Implement for anonymous functions
impl<Marker, F> IntoSystemSet<(SystemMarker, Marker)> for F
where
    F: IntoSystem<(), (), Marker> + Copy,
{
    fn into_system_set() -> SystemSet {
        SystemSet(vec![Sys::Anonymous(SystemId::of::<F::System>())])
    }

    fn into_systems() -> Systems {
        Systems(vec![F::into_metadata()])
    }
}

impl<T> IntoSystemSet<()> for T
where
    T: Reflected,
{
    fn into_system_set() -> SystemSet {
        SystemSet(vec![Sys::Named(StableId::from_typed::<T>())])
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct TupleMarker;

macro_rules! impl_system_collection {
    ($(($param: ident, $sys: ident)),*) => {
        /// Implement IntoSystemSet for all possible sets
        impl<$($param, $sys),*> IntoSystemSet<(TupleMarker, $($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoSystemSet<$param> + Copy),*
        {
            fn into_system_set() -> SystemSet {
                let mut systems = Vec::new();
                $(
                    systems.extend($sys::into_system_set().0);
                )*
                SystemSet(systems)
            }

            #[allow(non_snake_case)]
            fn into_systems() -> Systems {
                let mut systems = Vec::new();
                $(
                    systems.extend($sys::into_systems().0);
                )*
                Systems(systems)
            }
        }
    }
}

all_tuples!(impl_system_collection, 1, 20, P, S);

#[cfg(test)]
mod tests {
    use bevy_reflect::Reflect;
    use common::Param;

    extern crate alloc;
    use alloc::borrow::ToOwned;

    use super::*;
    use crate::prelude::Commands;

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

    fn into_system_sets<T, Marker>(_systems: T) -> Vec<Sys>
    where
        T: IntoSystemSet<Marker>,
    {
        T::into_system_set().0
    }

    fn into_systems<T, Marker>(_systems: T) -> Vec<System>
    where
        T: IntoSystemSet<Marker>,
    {
        T::into_systems().0
    }

    #[test]
    fn anonymous_system_into_system_sets() {
        fn system(mut _commands: Commands) {}
        let system_set = system;

        assert_eq!(
            into_system_sets(system_set),
            vec![Sys::Anonymous(system.get_system_id())]
        );
        assert_eq!(
            into_systems(system_set),
            vec![make_system(system, vec![Param::Command])]
        );
    }

    #[test]
    fn named_into_system_sets() {
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet;
        let system_set = NamedSet;

        assert_eq!(
            into_system_sets(system_set),
            vec![Sys::Named(StableId::from_typed::<NamedSet>())]
        );
        assert_eq!(into_systems(system_set), Vec::new());
    }

    #[test]
    fn system_tuple_into_system_sets() {
        fn system1() {}
        fn system2(mut _commands: Commands) {}
        #[derive(Reflect, Clone, Copy)]
        struct NamedSet;
        let system_set = (system1, system2, NamedSet);

        assert_eq!(
            into_system_sets(system_set),
            vec![
                Sys::Anonymous(system1.get_system_id()),
                Sys::Anonymous(system2.get_system_id()),
                Sys::Named(StableId::from_typed::<NamedSet>()),
            ]
        );
        assert_eq!(
            into_systems(system_set),
            vec![
                make_system(system1, vec![]),
                make_system(system2, vec![Param::Command]),
            ]
        );
    }
}
