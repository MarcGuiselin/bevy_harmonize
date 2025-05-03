use std::{
    io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tracing::info;

mod feature;
pub use feature::LoadedFeature;

use super::SchedulingError;

pub mod schedule;

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
pub struct LoadedMod {
    pub(super) manifest_hash: common::FileHash,
    module: wasmer::Module,
    features: Vec<LoadedFeature>,
}

impl PartialEq for LoadedMod {
    fn eq(&self, other: &Self) -> bool {
        self.manifest_hash == other.manifest_hash
    }
}

impl LoadedMod {
    /// Load a mod from a path. The path can be either:
    /// - a directory containing ".wasm" and ".manifest" files
    /// - any mod file as long as it has siblings with matching names
    pub async fn try_from_path<P>(path: P) -> LoadedModResult
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        info!("Loading mod from path: {:?}", path);

        // Either files are like this: "modname/.wasm" or "modname.wasm"
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_owned()
            .into_string()
            .unwrap();

        let directory = if file_name.is_empty() || file_name.starts_with(".") {
            path
        } else {
            path.parent()
                .ok_or(LoadingError::FileNotFound(path.to_owned(), None))?
        };

        let package_name = file_name.split('.').next().unwrap().to_owned();

        let manifest_path = directory.join(format!("{}.manifest", package_name));
        let manifest_bytes = async_fs::read(&manifest_path)
            .await
            .map_err(|err| LoadingError::FileNotFound(manifest_path, Some(err)))?;

        let wasm_path = directory.join(format!("{}.wasm", package_name));
        let wasm_bytes = async_fs::read(&wasm_path)
            .await
            .map_err(|err| LoadingError::FileNotFound(wasm_path, Some(err)))?;

        Self::try_from_bytes(manifest_bytes, wasm_bytes).await
    }

    async fn try_from_bytes(manifest_bytes: Vec<u8>, wasm_bytes: Vec<u8>) -> LoadedModResult {
        let (manifest, _) = bincode::decode_from_slice::<common::ModManifest, _>(
            &manifest_bytes[..],
            bincode::config::standard(),
        )
        .map_err(|_| LoadingError::InvalidManifest)?;

        let wasm_hash = common::FileHash::from_sha256(Sha256::digest(&wasm_bytes).into());
        if wasm_hash != manifest.wasm_hash {
            return Err(LoadingError::MissmatchingDependencies);
        }

        let manifest_hash = common::FileHash::from_sha256(Sha256::digest(&manifest_bytes).into());

        let store = wasmer::Store::default();
        let module = wasmer::Module::new(&store, wasm_bytes).map_err(LoadingError::InvalidWasm)?;

        let mut features = Vec::with_capacity(manifest.features.len());
        for feature in manifest.features.iter() {
            features.push(LoadedFeature::try_from_descriptor(feature)?);
        }

        Ok(Self {
            manifest_hash,
            module,
            features,
        })
    }
}

pub type LoadedModResult = Result<LoadedMod, LoadingError>;

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
pub enum LoadingError {
    FileNotFound(PathBuf, Option<io::Error>),
    InvalidManifest,
    InvalidWasm(wasmer::CompileError),
    MissmatchingDependencies,
    InvalidSchedule(common::StableId),
    SchedulingError(SchedulingError),
}
