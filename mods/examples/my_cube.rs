#![feature(const_trait_impl)]

use api::prelude::*;

pub const SCHEMA: Schema = Mod::new("My cube")
    .add_resource::<CountFrames>()
    .add_systems(Update, update_frame_count)
    .into_schema();

#[derive(Reflect, Default, Addressable)]
pub struct CountFrames(pub u32);

pub fn update_frame_count(mut resource: ResMut<CountFrames>) {
    resource.0 += 123456;
}
