#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{any::TypeId, fmt};

use bevy_reflect::{DynamicTypePath, TypeInfo, TypePathTable, Typed};
use bincode::{Decode, Encode};

mod schedule;
pub use schedule::*;

mod identifiers;
pub use identifiers::*;

mod type_signature;
pub use type_signature::*;

mod utils;
pub use utils::*;

/// Identify structs
#[derive(Encode, Decode, PartialEq, Eq, Hash, Clone)]
pub struct StableId {
    pub crate_name: String,
    // pub crate_version: String, // TODO: add to bevy_reflect?
    pub name: String,
}

impl StableId {
    pub fn new(crate_name: &str, name: &str) -> Self {
        StableId {
            crate_name: crate_name.to_string(),
            name: name.to_string(),
        }
    }

    pub fn from_typed<T>() -> StableId
    where
        T: Typed,
    {
        Self::from_type_info(T::type_info())
    }

    pub fn from_dynamic(dynamic: &impl DynamicTypePath) -> StableId {
        let crate_name = dynamic.reflect_crate_name().unwrap_or("unknown");
        let name = dynamic.reflect_short_type_path();
        StableId {
            crate_name: crate_name.to_string(),
            name: name.to_string(),
        }
    }

    pub fn from_type_info(type_info: &TypeInfo) -> StableId {
        Self::from_type_path_table(type_info.type_path_table())
    }

    pub fn from_type_path_table(path: &TypePathTable) -> StableId {
        let crate_name = path.crate_name().unwrap_or("unknown");
        let name = path.short_path();
        StableId {
            crate_name: crate_name.to_string(),
            name: name.to_string(),
        }
    }
}

impl fmt::Debug for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StableId(\"{}::{}\")", self.crate_name, self.name)
    }
}

/// Identify systems
#[derive(Encode, Decode, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct SystemId(u64);

// Custom hasher that just returns bytes
use core::hash::{Hash, Hasher};

impl SystemId {
    pub fn of<T: ?Sized + 'static>() -> Self {
        Self::from_type(TypeId::of::<T>())
    }

    pub fn from_type(id: TypeId) -> Self {
        #[allow(deprecated)]
        let mut hasher = core::hash::SipHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl fmt::Debug for SystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SystemId(\"{:x}\")", self.0)
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Hash)]
pub enum Param {
    Command,
    Res { mutable: bool, id: StableId },
    // TODO: Query, etc
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct FeatureDescriptor {
    pub name: String,
    pub resources: Vec<(StableId, Vec<u8>)>,
    pub schedules: Vec<schedule::ScheduleDescriptor>,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ModManifest {
    pub wasm_hash: FileHash,
    pub types: Vec<TypeSignature>,
    pub features: Vec<FeatureDescriptor>,
}

impl ModManifest {
    /// Get the list of all systems in the manifest in a deterministic order
    /// (based on the order of the features and schedules)
    pub fn systems(&self) -> Vec<&System> {
        let mut systems = Vec::new();
        for feature in &self.features {
            for schedule in &feature.schedules {
                for system in &schedule.schedule.systems {
                    if !systems.iter().any(|s: &&System| s.id == system.id) {
                        systems.push(system);
                    }
                }
            }
        }
        systems
    }
}

#[derive(Encode, Decode, PartialEq, Clone)]
pub struct FileHash([u8; 16]);

impl fmt::Debug for FileHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
