mod function_system;
mod params;
mod schedule;
mod system;
mod system_param;
mod system_set;

use core::ops::{Deref, DerefMut};

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

    /// Export system metadata
    fn into_metadata() -> common::System;
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
pub(crate) fn into_metadata<Marker, In, Out, S>(_system: S) -> common::System
where
    S: IntoSystem<In, Out, Marker>,
{
    S::into_metadata()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Commands;

    #[test]
    fn simple_system() {
        static mut RAN: bool = false;

        fn system() {
            unsafe {
                RAN = true;
            }
        }

        system.into_system().run(());
        assert!(unsafe { RAN }, "system did not run");

        let metadata = into_metadata(system);
        assert_eq!(
            metadata.name,
            "bevy_harmonize_api::ecs::system::tests::simple_system::system"
        );
    }

    #[test]
    fn system_with_param() {
        static mut RAN: bool = false;

        fn system(mut _commands: Commands) {
            unsafe {
                RAN = true;
            }
        }

        system.into_system().run(());
        assert!(unsafe { RAN }, "system did not run");

        let meta = into_metadata(system);
        assert_eq!(meta.params, [common::Param::Command]);

        assert_eq!(
            meta.name,
            "bevy_harmonize_api::ecs::system::tests::system_with_param::system"
        );
    }
}
