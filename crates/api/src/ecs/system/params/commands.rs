use std::marker::PhantomData;

use bevy_reflect::FromReflect;

use crate::ecs::system::{system_param::Params, SystemParam};

#[link(wasm_import_module = "bevy_harmonize")]
extern "C" {
    fn spawn_empty() -> u32;
}

pub struct Commands<'a>(
    // SystemParams should not be able to live outside a system
    PhantomData<&'a ()>,
);

impl<'a> SystemParam for Commands<'a> {
    type State = ();
    type Item<'state> = Commands<'state>;

    fn init_state() -> Self::State {
        ()
    }

    fn get_param<'state>(_state: &'state mut Self::State) -> Self::Item<'state> {
        Commands(PhantomData)
    }

    fn get_metadata() -> Params {
        vec![common::Param::Command]
    }
}

/// Similar to bevy_ecs::system::commands::Commands
impl<'a> Commands<'a> {
    pub fn spawn_empty(&mut self) -> EntityCommands<'a> {
        let id = unsafe { spawn_empty() };
        EntityCommands(id, PhantomData)
    }
}

pub struct EntityCommands<'a>(
    u32,
    // Lifetime must be restricted to within the system
    PhantomData<&'a ()>,
);

impl<'a> EntityCommands<'a> {
    // TODO: replace with insert<T: Bundle>(&mut self, bundle: T)
    pub fn insert_component(&mut self, _component: impl FromReflect) -> &mut Self {
        unimplemented!()
    }

    pub fn id(&self) -> Entity {
        Entity(self.0)
    }
}

/// Similar to bevy's Entity
#[derive(Debug)]
#[allow(dead_code)]
pub struct Entity(u32);
