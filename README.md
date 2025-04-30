# bevy_harmonize


**⚠️ This project is very much work-in-progress**

A modding system using wasm micro-modules. Keep Bevy's ergonomic design for your modders (and yourself 😏).

The idea is for mods to look something like this:

```rust
use api::prelude::*;

pub const SCHEMA: Schema = Mod::new("My cube")
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
