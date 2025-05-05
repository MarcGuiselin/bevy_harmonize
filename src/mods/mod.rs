use std::{future::Future, path::Path};

use anyhow::*;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::system::ResMut;
use bevy_ecs_macros::Resource;
use bevy_tasks::{block_on, poll_once, AsyncComputeTaskPool, Task};
use tracing::{error, info, warn};

mod engine;
use engine::Engine;

mod loaded;
use loaded::LoadedMod;

pub(crate) struct ModPlugin;

impl Plugin for ModPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mods>()
            .add_systems(Update, handle_loading_mods);
    }
}

#[derive(Resource, Default)]
pub struct Mods {
    engine: Engine,
    loading: Vec<Task<Result<LoadedMod>>>,
    loaded: Vec<Option<LoadedMod>>,
}

impl Mods {
    pub fn load_from_path<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let engine = self.engine.clone();
        let path = path.as_ref().to_owned();
        self.enque_loading(LoadedMod::try_from_path(engine, path))
    }

    fn enque_loading(&mut self, future: impl Future<Output = Result<LoadedMod>> + Send + 'static) {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(future);
        self.loading.push(task);
    }
}

fn handle_loading_mods(mut mods: ResMut<Mods>) {
    // Remove loaded tasks from loading
    let mut loaded = Vec::new();
    mods.loading.retain_mut(|task| {
        if let Some(result) = block_on(poll_once(task)) {
            loaded.push(result);
            false
        } else {
            true
        }
    });

    for loaded in loaded {
        match loaded {
            Result::Ok(loaded) => {
                let opt = Some(loaded);
                if mods.loaded.contains(&opt) {
                    warn!(
                        "Mod already loaded: {:#?}. Skipping.",
                        opt.as_ref().unwrap().manifest_hash
                    );
                } else {
                    info!("Mod loaded: {:#?}", opt.as_ref().unwrap());
                    mods.loaded.push(opt);
                }
            }
            Err(err) => {
                error!("Failed to load mod:\n{:?}", err);
            }
        }
    }
}
