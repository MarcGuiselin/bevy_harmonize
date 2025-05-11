use std::path::PathBuf;

use bevy::prelude::*;
use bevy_harmonize::prelude::*;
use bevy_harmonize_devtools::ModDevtoolsPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ModLoaderPlugin,
            ModDevtoolsPlugin {
                // Watches and builds the mods found the `./examples/mods` directory
                watch_dir: PathBuf::from("./examples/mods"),
                ..default()
            },
        ))
        .run();
}
