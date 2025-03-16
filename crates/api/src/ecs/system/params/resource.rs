use std::ops::{Deref, DerefMut};

use common::StableId;

use crate::ecs::{
    generic::ComponentId,
    system::{system_param::Params, SystemParam},
    Resource,
};

#[link(wasm_import_module = "bevy_harmonize")]
extern "C" {
    fn flag_component_changed(component_id: u32);
}

pub struct ResMut<'w, T>
where
    T: Resource,
{
    id: &'w ComponentId,
    changed: bool,
    value: &'w mut T,
}

impl<'a, T> SystemParam for ResMut<'a, T>
where
    T: Resource,
{
    type State = (ComponentId, T);
    type Item<'state> = ResMut<'state, T>;

    fn init_state() -> Self::State {
        unimplemented!()
    }

    fn get_param<'state>((id, value): &'state mut Self::State) -> Self::Item<'state> {
        ResMut {
            id,
            changed: false,
            value,
        }
    }

    fn get_metadata() -> Params {
        vec![common::Param::Res {
            mutable: false,
            id: StableId::from_typed::<T>(),
        }]
    }
}

impl<'w, T> Deref for ResMut<'w, T>
where
    T: Resource,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> AsRef<T> for ResMut<'w, T>
where
    T: Resource,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'w, T> DerefMut for ResMut<'w, T>
where
    T: Resource,
{
    #[inline]
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        self.value
    }
}

impl<'w, T> AsMut<T> for ResMut<'w, T>
where
    T: Resource,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'w, T> Drop for ResMut<'w, T>
where
    T: Resource,
{
    fn drop(&mut self) {
        if self.changed {
            unsafe { flag_component_changed(self.id.0) }
        }
    }
}
