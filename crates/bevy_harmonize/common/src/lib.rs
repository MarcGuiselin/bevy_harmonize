#![feature(const_type_name)]

use std::{
    any::TypeId,
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy_reflect::{TypeInfo, Typed};
use bitcode::{Decode, Encode};

mod schedule;
pub use schedule::*;

mod identifiers;
pub use identifiers::*;

mod utils;
pub use utils::*;

/// Identify structs
#[derive(Encode, Decode, PartialEq, Eq, Hash, Clone, Copy)]
pub struct StableId<'a> {
    pub crate_name: &'a str,
    // pub crate_version: &'a str, // TODO: add to bevy_reflect?
    pub name: &'a str,
}

impl<'a> StableId<'a> {
    pub fn from_typed<T>() -> StableId<'static>
    where
        T: Typed,
    {
        Self::from_type_info(T::type_info())
    }

    pub fn from_type_info(type_info: &TypeInfo) -> StableId<'static> {
        let path = type_info.type_path_table();
        let crate_name = path.crate_name().unwrap_or("unknown");
        let name = type_info.type_path_table().short_path();
        StableId { crate_name, name }
    }
}

impl<'a> StableId<'a> {
    pub fn to_owned(&self) -> OwnedStableId {
        OwnedStableId {
            crate_name: self.crate_name.to_owned(),
            name: self.name.to_owned(),
        }
    }
}

impl<'a> fmt::Debug for StableId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StableId(\"{}::{}\")", self.crate_name, self.name)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct OwnedStableId {
    pub crate_name: String,
    pub name: String,
}

impl OwnedStableId {
    pub fn from_typed<T>() -> OwnedStableId
    where
        T: Typed,
    {
        StableId::from_typed::<T>().to_owned()
    }

    pub fn from_type_info(type_info: &TypeInfo) -> OwnedStableId {
        StableId::from_type_info(type_info).to_owned()
    }
}

impl fmt::Debug for OwnedStableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OwnedStableId(\"{}::{}\")", self.crate_name, self.name)
    }
}

/// Identify systems
#[derive(Encode, Decode, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct SystemId(u64);

impl SystemId {
    pub fn of<T: ?Sized + 'static>() -> Self {
        Self::from_type(TypeId::of::<T>())
    }

    pub fn from_type(id: TypeId) -> Self {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        let result = hasher.finish();

        Self(result)
    }

    pub fn get_raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for SystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SystemId(\"{:x}\")", self.0)
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Hash, Copy)]
pub enum Param<'a> {
    Command,
    Res(StableId<'a>),
    // TODO: Query, Res, etc
}

impl Param<'_> {
    pub fn to_owned(&self) -> OwnedParam {
        match self {
            Param::Command => OwnedParam::Command,
            Param::Res(stable_id) => OwnedParam::Res(stable_id.to_owned()),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum OwnedParam {
    Command,
    Res(OwnedStableId),
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct FeatureDescriptor<'a> {
    pub name: &'a str,
    pub resources: Vec<(StableId<'a>, Vec<u8>)>,
    pub schedules: Vec<schedule::ScheduleDescriptor<'a>>,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ModManifest<'a> {
    pub wasm_hash: FileHash,
    pub features: Vec<FeatureDescriptor<'a>>,
}

#[derive(Encode, Decode, PartialEq)]
pub struct FileHash([u8; 16]);

impl std::fmt::Debug for FileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileHash(\"")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "\")")?;
        Ok(())
    }
}

impl FileHash {
    pub fn empty() -> Self {
        Self([0; 16])
    }

    pub fn from_sha256(bytes: [u8; 32]) -> Self {
        let mut hash = [0; 16];
        hash.copy_from_slice(&bytes[..16]);
        Self(hash)
    }
}
