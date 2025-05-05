use std::path::Path;

use anyhow::{Context as AnyhowContext, *};
use sha2::{Digest, Sha256};
use tracing::info;

mod feature;
pub use feature::LoadedFeature;

use super::engine::{Engine, Instance};

pub mod schedule;

#[derive(Debug)]
pub struct LoadedMod {
    pub(super) manifest_hash: common::FileHash,
    features: Vec<LoadedFeature>,
    instance: Instance,
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
    pub async fn try_from_path(engine: Engine, path: impl AsRef<Path>) -> Result<LoadedMod> {
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
                .ok_or(anyhow!("Failed to find file ../{:?}", path))?
        };

        let package_name = file_name.split('.').next().unwrap().to_owned();

        let manifest_path = directory.join(format!("{}.manifest", package_name));
        let manifest_bytes = async_fs::read(&manifest_path).await.map_err(|err| {
            anyhow!(
                "Failed to read manifest file {:?}: {:?}",
                manifest_path,
                err
            )
        })?;

        let wasm_path = directory.join(format!("{}.wasm", package_name));
        let wasm_bytes = async_fs::read(&wasm_path)
            .await
            .map_err(|err| anyhow!("Failed to read wasm file {:?}: {:?}", wasm_path, err))?;

        Self::try_from_bytes(engine, manifest_bytes, wasm_bytes)
            .await
            .with_context(|| format!("Failed to load mod from path: {:?}", path))
    }

    async fn try_from_bytes(
        engine: Engine,
        manifest_bytes: impl AsRef<[u8]>,
        wasm_bytes: impl AsRef<[u8]>,
    ) -> Result<LoadedMod> {
        let (manifest, _) = bincode::decode_from_slice::<common::ModManifest, _>(
            manifest_bytes.as_ref(),
            bincode::config::standard(),
        )
        .map_err(|_| anyhow!("Failed to parse manifest"))?;

        let wasm_hash = common::FileHash::from_sha256(Sha256::digest(&wasm_bytes).into());
        if wasm_hash != manifest.wasm_hash {
            bail!("Wasm hash does not match manifest");
        }

        let mut features = Vec::with_capacity(manifest.features.len());
        for feature in manifest.features.iter() {
            features.push(LoadedFeature::try_from_descriptor(feature)?);
        }

        let manifest_hash = common::FileHash::from_sha256(Sha256::digest(&manifest_bytes).into());

        let instance = Instance::new(&engine, wasm_bytes.as_ref())?;

        Ok(Self {
            manifest_hash,
            features,
            instance,
        })
    }
}
