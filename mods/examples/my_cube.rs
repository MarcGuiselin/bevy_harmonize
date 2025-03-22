use api::prelude::*;

mod __codegen {
    pub(crate) const fn __resolve_component_id<T>() -> usize
    where
        T: Sized,
    {
        1234500
    }

    pub(crate) const fn __resolve_address<T>() -> *mut T
    where
        T: Sized,
    {
        let alignment = align_of::<T>();
        if alignment > 128 {
            panic!("bevy harmonize can only ensure alignments up to 128");
        }

        1234500 as _
    }
}

#[derive(Reflect, Default, Addressable)]
pub struct CountFrames(pub u32);

pub fn update_frame_count(mut resource: ResMut<CountFrames>) {
    resource.0 += 123456;
}
