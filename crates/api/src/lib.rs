// Nightly has a weird issue when testing and enabling const_trait_impl via RUSTFLAGS
// It produces an error saying the feature const_trait_impl is not enabled, but when enabled warns that it is already enabled (due to RUSTFLAGS)
// Thus, to run tests it is necessary to set RUSTFLAGS="" and use the feature "test"
#![cfg_attr(feature = "test", feature(const_trait_impl))]
#![feature(const_trait_impl)]

#[cfg(all(feature = "generate_manifest", feature = "wasm_runtime"))]
compile_error!(
    "Features \"generate_manifest\" and \"wasm_runtime\" cannot be enabled at the same time"
);

#[path = "internal/mod.rs"]
pub mod __internal;

pub mod ecs;
pub mod schema;

pub mod prelude {
    pub use bevy_reflect::prelude::*;
    pub use bevy_reflect_derive::*;

    pub use crate::ecs::{
        system::{Commands, IntoSchedule, IntoSystemSet, ResMut},
        Addressable, Reflected, Resource,
    };
    pub use crate::schema::{Mod, Schema};

    // Schedules
    pub use common::{Start, Update};

    pub use derive::Addressable;
}
