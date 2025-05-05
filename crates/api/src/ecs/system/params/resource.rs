use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
extern crate alloc;
use alloc::vec;
use common::StableId;

use crate::ecs::{
    system::{system_param::Params, SystemParam},
    Resource,
};

pub struct ResMut<'w, T>
where
    T: Resource,
{
    changed: bool,
    phantom: PhantomData<&'w T>,
}

impl<'a, T> SystemParam for ResMut<'a, T>
where
    T: Resource,
{
    type State = ();
    type Item<'state> = ResMut<'state, T>;

    fn init_state() -> Self::State {
        ()
    }

    fn get_param<'state>(_: &'state mut Self::State) -> Self::Item<'state> {
        ResMut {
            changed: false,
            phantom: PhantomData,
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
        unsafe { &*T::PTR }
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
        unsafe { &mut *T::PTR }
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
            unsafe { crate::external::flag_component_changed(T::COMPONENT_ID) }
        }
    }
}
