use api::prelude::*;

#[derive(Reflect, Default)]
pub struct CountFrames(u32);

pub fn update_frame_count<const ID1: usize>(mut resource: ResMut<ID1, CountFrames>) {
    resource.0 += 123456;
}
