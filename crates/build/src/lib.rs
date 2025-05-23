#![allow(non_local_definitions)] // TODO: Fix downstream in bart

use anyhow::*;
use common::{ModManifest, RawWasmVec};
use postprocess::{transform_wasm, TypeAddress};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
use tracing::{info, warn};
use wasmtime::*;

mod command;
use command::CargoCommand;

mod fs_utils;
mod postprocess;
mod templates;

pub async fn build(
    release: bool,
    mods_directory: PathBuf,
    cargo_directory: PathBuf,
) -> Result<Vec<PathBuf>> {
    let start = Instant::now();
    info!("Building mods from {:?}", mods_directory);

    let mut sources = ModSource::from_dir(&mods_directory).await?;
    if sources.is_empty() {
        warn!("There are no mods to build");
        return Ok(Vec::new());
    }

    let dir = Directories::create(cargo_directory, release).await?;

    // Prepare codegen
    fs_utils::empty_dir_conditional(&dir.codegen, |path| {
        // Avoid deleting the empty crate which is kept version controled
        !path.ends_with("empty")
    })
    .await?;
    for source in sources.iter_mut() {
        source.codegen(&dir).await?;

        // Assuming mod manifests didn't change since last build, we might be able to compile everything with one single cargo build command
        // This might fail if no manifest was generated for this mod yet
        let _ = source.codegen_final(&dir).await;
    }

    // Try building everything in one go
    // If this fails, it's probably just be because manifests changed and thus the codegen is invalid
    if let Err(e) = cargo_build(
        &dir,
        sources
            .iter()
            .flat_map(|source| source.get_packages())
            .collect(),
        release,
    )
    .await
    {
        warn!("Initial cargo build ran into an error:\n{:?}", e);
        info!("Retrying building manifests only");

        // Build only the mod manifests
        cargo_build(
            &dir,
            sources
                .iter()
                .map(|source| source.get_manifest_export_package())
                .collect(),
            release,
        )
        .await?;
    } else {
        info!("Initial cargo build succeeded");
    }

    // Load manifest exports since all manifests export crates should have their wasm binaries generated by now
    for source in sources.iter_mut() {
        source.load_manifest(&dir).await?;
    }

    // Sources whose manifest changed need their codegen regenerated and export rebuilt
    if sources.iter().any(|source| !source.finished_codegen) {
        let packages = sources
            .iter()
            .filter(|source| !source.finished_codegen)
            .map(|source| source.get_systems_export_package())
            .collect();

        // Regenerate the codegen for the remaining sources
        for source in sources.iter_mut().filter(|source| !source.finished_codegen) {
            source.codegen_final(&dir).await?;
        }

        // Build the remaining systems export crates
        cargo_build(&dir, packages, release).await?;
    }

    // Delete old wasm files
    fs_utils::empty_dir_conditional(&dir.dest, |path| path.ends_with(".wasm")).await?;

    // Move the generated wasm files to the build directory
    let mut wasm_files = Vec::with_capacity(sources.len());
    for source in sources.iter_mut() {
        wasm_files.push(source.finish(&dir).await?);
    }

    let duration = start.elapsed();
    info!("Successfully built mods {:?} in {:?}", wasm_files, duration);

    Ok(wasm_files)
}

struct Directories {
    cargo_directory: PathBuf,
    dev_mode: &'static str,
    codegen: PathBuf,
    dest: PathBuf,
    wasm_dest: PathBuf,
}

const WASM_TARGET: &str = "wasm32-unknown-unknown";

impl Directories {
    const TARGET_DIR: &str = "target";
    const BUILD_DIR: &str = "bevy-harmonize-build";
    const CODEGEN_DIR: &str = "codegen/crates";

    async fn create(cargo_directory: PathBuf, release: bool) -> Result<Self> {
        let cargo_directory = dunce::canonicalize(cargo_directory)?;
        let target = cargo_directory.join(Self::TARGET_DIR);
        let build = target.join(Self::BUILD_DIR);
        let codegen = cargo_directory.join(Self::CODEGEN_DIR);
        let dev_mode = if release { "release" } else { "debug" };
        let dest = build.join(dev_mode);
        let wasm_dest = target.join(WASM_TARGET).join(dev_mode);

        fs_utils::create_dir_all(&codegen).await?;
        fs_utils::create_dir_all(&dest).await?;

        Ok(Self {
            cargo_directory,
            codegen,
            dev_mode,
            dest,
            wasm_dest,
        })
    }
}

/// A source file for a mod
#[derive(Debug)]
pub struct ModSource {
    source: PathBuf,
    name: String,
    types: Vec<TypeAddress>,
    finished_codegen: bool,
    manifest: Option<ModManifest>,
}

impl ModSource {
    async fn from_dir(path: &PathBuf) -> Result<Vec<Self>> {
        let files = fs_utils::list_files_in_dir(path).await?;
        let mut sources = Vec::new();
        for file in files {
            if file.extension().map_or(false, |ext| ext == "rs") {
                let path = dunce::realpath(file)?;
                sources.push(Self::new(path));
            }
        }
        Ok(sources)
    }

    fn new(source: PathBuf) -> Self {
        let path_hash: [u8; 32] = Sha256::digest(source.as_os_str().as_encoded_bytes()).into();
        let package_suffix: String = path_hash[..4]
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();

        let name = source.file_stem().unwrap().to_str().unwrap();
        let package_name = format!(
            "{}_{}",
            name.to_lowercase().replace(" ", "_"),
            &package_suffix
        );

        Self {
            source,
            name: package_name,
            types: Vec::new(),
            finished_codegen: false,
            manifest: None,
        }
    }

    const WASM: &str = "wasm";
    const WASM_DEBUG: &str = "wasm.wat";
    const MANIFEST: &str = "manifest";
    const MANIFEST_DEBUG: &str = "manifest.txt";
    const SOURCE: &str = "_source";
    const IMPORTS: &str = "_imports";
    const EXPORT_MANIFEST: &str = "_export_manifest";
    const EXPORT_SYSTEMS: &str = "_export_systems";

    fn get_packages(&self) -> Vec<String> {
        let mut packages = vec![self.get_manifest_export_package()];
        if self.finished_codegen {
            packages.push(self.get_systems_export_package());
        }
        packages
    }

    fn get_manifest_export_package(&self) -> String {
        format!("{}{}", self.name, Self::EXPORT_MANIFEST)
    }

    fn get_systems_export_package(&self) -> String {
        format!("{}{}", self.name, Self::EXPORT_SYSTEMS)
    }

    /// Performs the initial codegen, before mod manifests are necessarily resolved
    async fn codegen(&self, dir: &Directories) -> Result<()> {
        let name = &self.name;
        let file_name = self.source.file_name().unwrap().to_str().unwrap();
        let source_file = &self.source.to_str().unwrap().replace("\\", "/");
        let modloader_version = env!("CARGO_PKG_VERSION");
        let dev_mode = dir.dev_mode;

        let imports_path = dir.codegen.join(format!("{}{}", name, Self::IMPORTS));
        fs_utils::write_template(
            imports_path.join("Cargo.toml"),
            templates::ImportsCargo {
                file_name,
                modloader_version,
                dev_mode,
                name,
            },
        )
        .await?;
        fs_utils::write_template(
            imports_path.join("lib.rs"),
            templates::ImportsLib {
                // Empty since we don't know the contents of the manifest yet
                components: &[],
            },
        )
        .await?;

        let source_pkg_path = dir.codegen.join(format!("{}{}", name, Self::SOURCE));
        fs_utils::write_template(
            source_pkg_path.join("Cargo.toml"),
            templates::SourceCargo {
                file_name,
                modloader_version: env!("CARGO_PKG_VERSION"),
                dev_mode,
                name,
                source_file,
            },
        )
        .await?;

        let export_manifest_path = dir
            .codegen
            .join(format!("{}{}", name, Self::EXPORT_MANIFEST));
        fs_utils::write_template(
            export_manifest_path.join("Cargo.toml"),
            templates::ExportsManifestCargo {
                file_name,
                modloader_version,
                dev_mode,
                name,
            },
        )
        .await?;
        fs_utils::write_template(
            export_manifest_path.join("lib.rs"),
            templates::ExportsManifestLib {},
        )
        .await?;

        Ok(())
    }

    /// Reads the manifest binary and generates codegen
    async fn codegen_final(&mut self, dir: &Directories) -> Result<()> {
        let name = &self.name;
        let file_name = self.source.file_name().unwrap().to_str().unwrap();
        let modloader_version = env!("CARGO_PKG_VERSION");
        let dev_mode = dir.dev_mode;

        // Try loading manifest from the previous build
        if self.manifest.is_none() {
            let manifest_path = dir.dest.join(&self.name).with_extension(Self::MANIFEST);
            let manifest_bytes = fs_utils::read(&manifest_path).await?;
            let (manifest, _) = bincode::decode_from_slice::<ModManifest, _>(
                &manifest_bytes[..],
                bincode::config::standard(),
            )
            .with_context(|| format!("Failed to read manifest file: {:?}", manifest_path))?;

            self.manifest = Some(manifest);
        }

        let manifest = self.manifest.as_ref().unwrap();

        let systems: Vec<_> = manifest
            .systems()
            .iter()
            .enumerate()
            .map(|(id, system)| templates::ExportsSystem {
                id: id as u32,
                name: &system.name,
            })
            .collect();

        let export_systems_path = dir
            .codegen
            .join(format!("{}{}", name, Self::EXPORT_SYSTEMS));
        fs_utils::write_template(
            export_systems_path.join("Cargo.toml"),
            templates::ExportsSystemsCargo {
                file_name,
                modloader_version,
                dev_mode,
                name,
            },
        )
        .await?;
        fs_utils::write_template(
            export_systems_path.join("lib.rs"),
            templates::ExportsSystemsLib {
                systems: &systems[..],
            },
        )
        .await?;

        self.types = TypeAddress::from_type_signatures(manifest.types.clone().into_iter());
        let components: Vec<_> = self
            .types
            .iter()
            .enumerate()
            .map(|(id, type_address)| {
                let sid = type_address.signature.stable_id();
                templates::ImportsComponent {
                    crate_name: sid.crate_name,
                    name: sid.name,
                    id: id as u32,
                    address: type_address.address.start,
                }
            })
            .collect();

        let imports_path = dir.codegen.join(format!("{}{}", name, Self::IMPORTS));
        fs_utils::write_template(
            imports_path.join("lib.rs"),
            templates::ImportsLib {
                components: &components[..],
            },
        )
        .await?;

        self.finished_codegen = true;

        Ok(())
    }

    /// Uses the built wasm manifest export to generate the mod manifest
    async fn load_manifest(&mut self, dir: &Directories) -> Result<()> {
        let package_name = self.get_manifest_export_package();
        let path = dir.wasm_dest.join(&package_name).with_extension(Self::WASM);

        let mut config = Config::new();
        config.cache_config_load_default()?;
        config.parallel_compilation(true);

        struct Context {
            panic: Option<RawWasmVec>,
        }

        let engine = Engine::new(&config)?;
        let mut store = Store::new(&engine, Context { panic: None });
        let module = Module::from_file(&engine, path)?;

        let mut linker = Linker::new(&engine);

        linker.func_wrap(
            "bevy_harmonize",
            "panic",
            |mut caller: Caller<Context>, ptr: u32, len: u32| -> Result<()> {
                let ptr = ptr as usize;
                let len = len as usize;

                caller.data_mut().panic = Some(RawWasmVec { ptr, len });

                // Trap
                Err(anyhow!("Panic in wasm module"))
            },
        )?;

        linker.define_unknown_imports_as_traps(&module)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .with_context(|| "Error instantiating wasm module for manifest")?;

        let run = instance.get_typed_func::<(), u64>(&mut store, "run")?;
        let result = run.call(&mut store, ()).map_err(|e| {
            if let Some(panic) = store.data().panic {
                let memory = instance.get_memory(&mut store, "memory").unwrap();
                let bytes = &memory.data(&store)[panic.into_range()];
                let message = String::from_utf8_lossy(&bytes[..]);
                anyhow!(
                    "Panic in wasm module while generating manifest.\n{}",
                    message
                )
            } else {
                e
            }
        })?;
        let vec = RawWasmVec::from(result);

        let memory = instance.get_memory(&mut store, "memory").unwrap();
        let manifest_bytes = &memory.data(&store)[vec.into_range()];

        if manifest_bytes.is_empty() {
            bail!("Manifest bytes are empty");
        }

        let (manifest, _) = bincode::decode_from_slice::<ModManifest, _>(
            manifest_bytes,
            bincode::config::standard(),
        )?;

        if self.manifest.as_ref() != Some(&manifest) {
            if self.manifest.is_some() {
                warn!("Manifest changed for {}", self.name);
            }
            self.manifest = Some(manifest);

            // Codegen is invalid since the previous manifest used to create it is now invalid
            self.finished_codegen = false;
        }

        Ok(())
    }

    async fn finish(&mut self, dir: &Directories) -> Result<PathBuf> {
        let package_name = self.get_systems_export_package();
        let src = dir.wasm_dest.join(package_name).with_extension(Self::WASM);
        let dest = dir.dest.join(&self.name).with_extension(Self::WASM);

        let bytes = transform_wasm(&src, &self.types).await?;

        fs_utils::write(&dest, &bytes).await?;

        let printed = wasmprinter::print_bytes(&bytes)?;
        fs_utils::write(dest.with_extension(Self::WASM_DEBUG), printed).await?;

        let mut manifest = self.manifest.take().expect("Manifest should be loaded");
        manifest.wasm_hash = common::FileHash::from_sha256(Sha256::digest(&bytes).into());

        let as_string = format!("{:#?}", manifest);
        let path = dest.with_extension(Self::MANIFEST_DEBUG);
        fs_utils::write(&path, as_string).await?;

        let encoded_manifest = bincode::encode_to_vec(manifest, bincode::config::standard())?;
        let path = dest.with_extension(Self::MANIFEST);
        fs_utils::write(&path, encoded_manifest).await?;

        Ok(dest)
    }
}

async fn cargo_build(dir: &Directories, packages: Vec<String>, release: bool) -> Result<()> {
    let mut command = CargoCommand::new("build")?;
    command
        .packages(packages.into_iter())
        .current_dir(dir.cargo_directory.clone())
        .target(WASM_TARGET);
    //.arg("build-std=panic_abort,std")
    //.env("RUSTFLAGS", "-C link-arg=--import-memory");

    if release {
        command.arg("--release");
    }

    command.spawn().await?;

    Ok(())
}
