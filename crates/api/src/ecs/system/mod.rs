mod function_system;
mod params;
mod schedule;
mod system;
mod system_param;
mod system_set;

use core::{
    any::{type_name, TypeId},
    ops::{Deref, DerefMut},
};

pub use function_system::FunctionSystem;
pub use params::*;
pub use schedule::{IntoSchedule, Schedule};
pub use system::System;
pub use system_param::SystemParam;
pub use system_set::IntoSystemSet;

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system with input `{In}` and output `{Out}`",
    label = "invalid system"
)]
pub trait IntoSystem<In, Out, Marker>
where
    Self: Sized,
{
    /// The type of [`System`] that this instance converts into.
    type System: System<In = In, Out = Out>;

    /// Some optional state required for initializing a system (usually for system_params)
    type State;

    /// Turns this value into its corresponding [`System`].
    fn into_system(self) -> Self::System;

    /// Turns this value into its corresponding [`System`] with the provided state
    ///
    /// SAFETY: Valid State must be provided
    unsafe fn into_system_with_state(self, state: Self::State) -> Self::System;

    /// Export system metadata
    fn into_metadata() -> common::System;

    /// Get the [`TypeId`] of the [`System`] produced after calling [`into_system`](`IntoSystem::into_system`).
    #[inline]
    fn get_system_id(&self) -> common::SystemId {
        common::SystemId::from_type(TypeId::of::<Self::System>())
    }

    #[inline]
    fn get_name(&self) -> &'static str {
        let full_name = type_name::<Self::System>();
        let name = extract_system_name(full_name);
        assert!(
            name.len() > 0,
            "System name is being parsed incorrectly\n   Input: {full_name}"
        );
        name
    }
}

/// Takes a full quantified type name and extracts the system name from it.
fn extract_system_name(original: &'static str) -> &'static str {
    assert!(original.len() >= 4, "String too small to be a system name");
    // Simplify deeply nested types (always end in >)
    if original.chars().last() == Some('>') {
        let start_pos = original
            .rfind(' ')
            .map(|start_pos| start_pos + 1usize)
            .unwrap_or(0usize);
        &original[start_pos..original.len() - 1]
    } else {
        original
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
pub struct In<In>(pub In);

impl<T> Deref for In<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for In<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Commands;

    fn into_metadata<T, Marker>(_system: T) -> common::System
    where
        T: IntoSystem<(), (), Marker>,
    {
        T::into_metadata()
    }

    #[test]
    fn simple_system() {
        static mut RAN: bool = false;

        fn sys() {
            unsafe {
                RAN = true;
            }
        }

        let mut system = IntoSystem::into_system(sys);
        assert_eq!(
            system.name(),
            "bevy_harmonize_api::ecs::system::tests::simple_system::sys"
        );
        system.run(());
        assert!(unsafe { RAN }, "system did not run");
    }

    #[test]
    fn system_with_param() {
        static mut RAN: bool = false;

        fn sys(mut _commands: Commands) {
            unsafe {
                RAN = true;
            }
        }

        let mut system = IntoSystem::into_system(sys);
        assert_eq!(
            system.name(),
            "bevy_harmonize_api::ecs::system::tests::system_with_param::sys"
        );
        system.run(());
        assert!(unsafe { RAN }, "system did not run");

        let meta = into_metadata(sys);
        assert_eq!(meta.params, [common::Param::Command]);
        assert_eq!(meta.id, sys.get_system_id());
    }
}
