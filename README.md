# Bevy Harmonize

A modding system using wasm micro-modules. Keep Bevy's ergonomic design for your modders (and yourself 😏).

**⚠️ This project is very much work-in-progress**

The idea is for mods to look something like this:

```rust
use api::prelude::*;

pub const SCHEMA: Schema = Mod::new("My frame counting mod")
    .add_resource::<CountFrames>()
    .add_systems(Update, update_frame_count)
    .into_schema();

#[derive(Reflect, Default, Addressable)]
pub struct CountFrames(pub u32);

pub fn update_frame_count(
    mut frames: ResMut<CountFrames>
) {
    frames.0 += 1;
    info!("Frame count: {}", frames.0);
}
```

This will eventually come with first class support for:

- Hot reloading
- Inline dependency management via uris
- Configurable mod permissions
- Authentication and mod signing
- Automatic updates
- And more yet to come!
