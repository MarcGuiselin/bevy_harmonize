use std::path::PathBuf;

use async_channel::Receiver;
use bevy_app::{App, Plugin, PostUpdate, PreStartup};
use bevy_ecs::{
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_ecs_macros::Resource;
use bevy_harmonize_build::build;
use bevy_tasks::{block_on, poll_once, AsyncComputeTaskPool, Task};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info};

use bevy_harmonize::prelude::*;

/// A plugin that adds modding development tools such as hot reloading
pub struct ModDevtoolsPlugin {
    /// The relative path to cargo, used to locate the target directory for the mod build.
    ///
    /// Defaults to the current working directory.
    pub cargo_dir: PathBuf,

    /// The relative path to the mods directory, used to locate the mod files.
    ///
    /// Defaults to `./mods`.
    pub watch_dir: PathBuf,
}

impl Default for ModDevtoolsPlugin {
    fn default() -> Self {
        Self {
            cargo_dir: PathBuf::from("."),
            watch_dir: PathBuf::from("./mods"),
        }
    }
}

impl Plugin for ModDevtoolsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BuildSettings {
            cargo_dir: self.cargo_dir.clone(),
            watch_dir: self.watch_dir.clone(),
        })
        .init_resource::<BuildTask>()
        .add_systems(PreStartup, update_build)
        .add_systems(PostUpdate, update_build);
    }
}

#[derive(Resource)]
struct BuildSettings {
    cargo_dir: PathBuf,
    watch_dir: PathBuf,
}

#[derive(Resource)]
struct BuildTask {
    compute: Option<Task<anyhow::Result<Vec<PathBuf>>>>,

    /// Indicates that there were one or more file changes
    trigger_build: Receiver<()>,

    _watcher: RecommendedWatcher,
}

impl FromWorld for BuildTask {
    fn from_world(world: &mut World) -> Self {
        let settings = world.get_resource::<BuildSettings>().unwrap();
        let (sender, receiver) = async_channel::bounded(1);

        // Rebuild at least once on startup
        sender.try_send(()).unwrap();

        let event_handler = move |event: Result<notify::Event, notify::Error>| match event {
            Ok(event) => match event.kind {
                notify::EventKind::Create(..)
                | notify::EventKind::Modify(..)
                | notify::EventKind::Remove(..) => {
                    let _ = sender.try_send(());
                }
                _ => {}
            },
            Err(err) => error!("Mod build file watcher error: {:?}", err),
        };
        let config = Default::default();
        let mut watcher = RecommendedWatcher::new(event_handler, config)
            .expect("Failed to create filesystem watcher.");

        watcher
            .watch(&settings.watch_dir, RecursiveMode::Recursive)
            .expect("Failed to watch path");

        Self {
            compute: None,
            trigger_build: receiver,
            _watcher: watcher,
        }
    }
}

fn update_build(settings: Res<BuildSettings>, mut task: ResMut<BuildTask>, mut mods: ResMut<Mods>) {
    // Check on the active build task
    if let Some(compute) = &mut task.compute {
        match block_on(poll_once(compute)) {
            Some(Ok(files)) => {
                for file in files {
                    mods.load_from_path(&file);
                }

                task.compute = None;
            }
            Some(Err(err)) => {
                error!("Error when building mods\n{:?}", err);

                task.compute = None;
            }
            None => {}
        }
    }

    // Initialize a new task when the previous one is finished
    if task.trigger_build.try_recv().is_ok() && task.compute.is_none() {
        info!("trigger_build!");

        let release = true; // !cfg!(debug_assertions);
        let mods_directory = settings.watch_dir.clone();
        let cargo_directory = settings.cargo_dir.clone();

        let future = build(release, mods_directory, cargo_directory);
        task.compute
            .replace(AsyncComputeTaskPool::get().spawn(future));
    }
}
