use const_vec::ConstVec;

extern crate alloc;
use alloc::vec::Vec;

use bevy_reflect::TypeInfo;

mod a_mod;
pub use a_mod::Mod;

use crate::ecs::system::Schedule;

#[derive(Debug, Clone, Copy)]
pub struct Schema {
    pub(crate) name: Option<&'static str>,
    pub(crate) types: ConstVec<InnerType, 1024>,
    pub(crate) resources: ConstVec<(fn() -> &'static TypeInfo, fn() -> Vec<u8>), 128>,
    pub(crate) schedules: ConstVec<(fn() -> &'static TypeInfo, Schedule), 128>,
}

impl Schema {
    pub const fn new() -> Self {
        Self {
            name: None,
            types: ConstVec::new(),
            resources: ConstVec::new(),
            schedules: ConstVec::new(),
        }
    }

    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    pub const fn types(&self) -> Types {
        Types {
            next: 0,
            getters: self.types.into_slice(),
        }
    }

    pub const fn resources(&self) -> Resources {
        Resources {
            next: 0,
            getters: self.resources.into_slice(),
        }
    }

    pub const fn schedules(&self) -> Schedules {
        Schedules {
            next: 0,
            getters: self.schedules.into_slice(),
        }
    }
}

pub struct Types<'a> {
    next: usize,
    getters: &'a [InnerType],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InnerType {
    pub(crate) getter: fn() -> &'static TypeInfo,
    pub(crate) size: usize,
    pub(crate) align: usize,
}

impl Into<Type> for &InnerType {
    fn into(self) -> Type {
        Type {
            info: (self.getter)(),
            size: self.size,
            align: self.align,
        }
    }
}

pub struct Type {
    pub info: &'static TypeInfo,
    pub size: usize,
    pub align: usize,
}

impl<'a> Iterator for Types<'a> {
    type Item = Type;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.getters.get(self.next).map(|inner| inner.into());
        self.next += 1;
        current
    }
}

pub struct Resources<'a> {
    next: usize,
    getters: &'a [(fn() -> &'static TypeInfo, fn() -> Vec<u8>)],
}

impl<'a> Iterator for Resources<'a> {
    type Item = (&'static TypeInfo, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self
            .getters
            .get(self.next)
            .map(|(getter1, getter2)| (getter1(), getter2()));
        self.next += 1;
        current
    }
}

pub struct Schedules<'a> {
    next: usize,
    getters: &'a [(fn() -> &'static TypeInfo, Schedule)],
}

impl<'a> Iterator for Schedules<'a> {
    type Item = (&'static TypeInfo, common::Schedule<'static>);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self
            .getters
            .get(self.next)
            .map(|(getter, schedule)| (getter(), schedule.build()));
        self.next += 1;
        current
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_check_size() {
        // Assume any size over 1MB is too big
        assert!(size_of::<Schema>() < 1024 * 1024);
    }
}
