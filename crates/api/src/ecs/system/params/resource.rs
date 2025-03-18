use std::ops::{Deref, DerefMut};

use common::StableId;

use crate::ecs::{
    system::{system_param::Params, SystemParam},
    Resource,
};

#[link(wasm_import_module = "bevy_harmonize")]
extern "C" {
    fn flag_component_changed(component_id: usize);
}

pub struct ResMut<'w, const IDEN: usize, T>
where
    T: Resource,
{
    changed: bool,
    value: &'w mut T,
}

impl<'a, const IDEN: usize, T> SystemParam for ResMut<'a, IDEN, T>
where
    T: Resource,
{
    type State = ();
    type Item<'state> = ResMut<'state, IDEN, T>;

    fn init_state() -> Self::State {
        ()
    }

    fn get_param<'state>(_: &'state mut Self::State) -> Self::Item<'state> {
        let ptr = const { IDEN } as *mut T;
        ResMut {
            changed: false,
            // SAFETY: It is expected a valid pointer was provided to IntoSystem::into_system_with_state
            value: unsafe { &mut *ptr },
        }
    }

    fn get_metadata() -> Params {
        vec![common::Param::Res {
            mutable: false,
            id: StableId::from_typed::<T>(),
        }]
    }
}

impl<'w, const IDEN: usize, T> Deref for ResMut<'w, IDEN, T>
where
    T: Resource,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, const IDEN: usize, T> AsRef<T> for ResMut<'w, IDEN, T>
where
    T: Resource,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'w, const IDEN: usize, T> DerefMut for ResMut<'w, IDEN, T>
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

impl<'w, const IDEN: usize, T> AsMut<T> for ResMut<'w, IDEN, T>
where
    T: Resource,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'w, const IDEN: usize, T> Drop for ResMut<'w, IDEN, T>
where
    T: Resource,
{
    fn drop(&mut self) {
        if self.changed {
            unsafe { flag_component_changed(const { IDEN }) }
        }
    }
}
