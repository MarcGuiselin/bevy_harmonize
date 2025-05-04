#![feature(const_trait_impl)]
#![no_std]

pub(crate) mod external;

pub mod allocator;
pub mod ecs;
pub mod panic;
pub mod schema;

pub mod prelude {
    pub use bevy_reflect::prelude::*;
    pub use bevy_reflect_derive::*;
    pub use tracing::{debug, error, info, trace, warn};

    pub use crate::ecs::{
        system::{Commands, IntoSchedule, IntoSystem, IntoSystemSet, ResMut},
        Addressable, Reflected, Resource,
    };
    pub use crate::schema::{Mod, Schema};

    // Schedules
    pub use common::{Start, Update};

    pub use derive::Addressable;
}
