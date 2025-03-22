#![allow(non_local_definitions)] // TODO: Fix downstream in bart

use async_process::Stdio;
use async_std::{
    io::{prelude::BufReadExt, BufReader, Read},
    stream::StreamExt,
    task::spawn,
};
use bevy_utils::tracing::{error, info};
use futures_concurrency::prelude::*;
use rancor::{fail, ResultExt};
use sha2::{Digest, Sha256};
use std::{error::Error, path::PathBuf, time::Instant};

mod command;
use command::CargoCommand;

mod fs_utils;

const TARGET_DIR: &str = "target";
const BUILD_DIR: &str = "bevy-harmonize-build";
const TEMP_DIR: &str = "temp";
const CODEGEN_DIR: &str = "codegen/crates";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

pub async fn build<E>(
    release: bool,
    mods_directory: PathBuf,
    cargo_directory: PathBuf,
) -> Result<Vec<PathBuf>, E>
where
    E: rancor::Source,
{
    let start = Instant::now();
    println!("Building mods from {:?}", mods_directory);

    let sources = ModSource::from_dir(&mods_directory).await?;
    if sources.is_empty() {
        println!("There are no mods to build");
        return Ok(Vec::new());
    }

    println!("{:?}", &sources);

    let target_dir = cargo_directory.join(TARGET_DIR);
    let build_dir = target_dir.join(BUILD_DIR);
    let dev_mode = if release { "release" } else { "debug" };
    let codegen_dir = cargo_directory.join(CODEGEN_DIR);

    // Prepare codegen
    fs_utils::empty_dir_conditional(&codegen_dir, |path| {
        // Avoid deleting the empty crate which is kept version controled
        !path.ends_with("empty")
    })
    .await?;
    for source in sources.iter() {
        source.codegen(&codegen_dir, &dev_mode).await?;
    }

    // Build a debug release of mods for manifest generation
    let packages: Vec<String> = sources
        .iter()
        .map(|result| result.get_package_name())
        .collect();
    build_raw(
        codegen_dir.clone(),
        packages.clone(),
        BuildType::GenerateManifest,
    )
    .await?;

    // Move generated wasm files to a temporary directory
    let temp_dir = build_dir.join(TEMP_DIR);
    fs_utils::create_dir_all_empty(&temp_dir).await?;
    let codegen_target_dir = codegen_dir.join(TARGET_DIR).join(WASM_TARGET);
    let codegen_build_dir = codegen_target_dir.join("debug");
    for package in packages.iter() {
        let filename = format!("{}.wasm", package);
        let src = codegen_build_dir.join(&filename);
        let dst = temp_dir.join(&filename);
        fs_utils::rename(src, dst).await?;
    }

    // 1. Generate the manifest for each mod
    let generate_manifests_fut = generate_manifests(temp_dir.clone(), packages.clone());

    // 2. Generate the wasm files for each mod
    let generate_wasm_fut = generate_wasm(release, codegen_dir.clone(), packages.clone());

    // Do 1 and 2 concurrently
    let (result1, result2) = (generate_manifests_fut, generate_wasm_fut).join().await;
    let encoded_manifests = result1?;
    result2?;

    // Do final processing of manifest and write files to target directory
    let codegen_build_dir = codegen_target_dir.join(dev_mode);
    let dest_dir = build_dir.join(dev_mode);
    packages
        .clone()
        .into_iter()
        .zip(encoded_manifests)
        .collect::<Vec<_>>()
        .into_co_stream()
        .map(|(package, encoded_manifest)| {
            final_processing(
                package,
                encoded_manifest,
                codegen_build_dir.clone(),
                dest_dir.clone(),
            )
        })
        .collect::<Vec<Result<(), _>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<()>, _>>()?;

    let duration = start.elapsed();
    info!("Successfully built mods {:?} in {:?}", packages, duration);

    let result = sources
        .iter()
        .map(|source| {
            let package_name = source.get_package_name();
            let wasm_file = dest_dir.join(format!("{}.wasm", package_name));
            wasm_file
        })
        .collect();
    Ok(result)
}

/// A source file for a mod
#[derive(Clone, Debug)]
pub struct ModSource(PathBuf);

impl ModSource {
    async fn from_dir<E>(path: &PathBuf) -> Result<Vec<Self>, E>
    where
        E: rancor::Source,
    {
        let files = fs_utils::list_files_in_dir(path).await?;
        let mut sources = Vec::new();
        for file in files {
            if file.extension().map_or(false, |ext| ext == "rs") {
                let path = dunce::realpath(file).into_error()?;
                sources.push(Self(path));
            }
        }
        Ok(sources)
    }

    async fn codegen<E>(&self, path: &PathBuf, dev_mode: &str) -> Result<(), E>
    where
        E: rancor::Source,
    {
        let file_name = self.0.file_name().unwrap().to_str().unwrap();
        let package_name = self.get_package_name();
        let source_file = self.0.to_str().unwrap().replace("\\", "/");
        let contents = format!(
            "{}",
            &CargoMod {
                file_name,
                modloader_version: env!("CARGO_PKG_VERSION"),
                dev_mode,
                package_name: &package_name,
                source_file: &source_file,
            }
        );

        let crate_dir = path.join(&package_name);
        fs_utils::create_dir_all(&crate_dir).await?;
        fs_utils::write(crate_dir.join("Cargo.toml"), contents).await?;

        Ok(())
    }

    fn get_package_name(&self) -> String {
        let path_hash: [u8; 32] = Sha256::digest(self.0.as_os_str().as_encoded_bytes()).into();
        let package_suffix: String = path_hash[..4]
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();

        let name = self.0.file_stem().unwrap().to_str().unwrap();
        format!(
            "{}_{}",
            name.to_lowercase().replace(" ", "_"),
            &package_suffix
        )
    }
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/mod.toml"]
struct CargoMod<'a> {
    file_name: &'a str,
    modloader_version: &'a str,
    dev_mode: &'a str,
    package_name: &'a str,
    source_file: &'a str,
}

async fn generate_manifests<E>(temp_dir: PathBuf, packages: Vec<String>) -> Result<Vec<Vec<u8>>, E>
where
    E: rancor::Source,
{
    packages
        .into_co_stream()
        .map(|package| {
            let path = temp_dir.join(format!("{}.wasm", package));
            wasm_export_encoded_manifest(path)
        })
        .collect::<Vec<Result<Vec<u8>, _>>>()
        .await
        .into_iter()
        .collect()
}

async fn generate_wasm<E>(release: bool, directory: PathBuf, packages: Vec<String>) -> Result<(), E>
where
    E: rancor::Source,
{
    let build_type = if release {
        BuildType::Release
    } else {
        BuildType::Debug
    };
    build_raw(directory.clone(), packages.clone(), build_type).await
}

async fn final_processing<E>(
    package: String,
    encoded_manifest: Vec<u8>,
    codegen_build_dir: PathBuf,
    dest_dir: PathBuf,
) -> Result<(), E>
where
    E: rancor::Source,
{
    let wasm_path = codegen_build_dir.join(format!("{}.wasm", package));
    let wasm_bytes = fs_utils::read(&wasm_path).await?;
    let wasm_hash = common::FileHash::from_sha256(Sha256::digest(&wasm_bytes).into());

    let common::ModManifest {
        wasm_hash: _,
        types,
        features,
    } = bitcode::decode(&encoded_manifest).unwrap();
    let manifest = common::ModManifest {
        wasm_hash,
        types,
        features,
    };

    let as_string = format!("{:#?}", manifest);
    let path = dest_dir.join(format!("{}.manifest.txt", package));
    fs_utils::write(&path, as_string).await?;

    let encoded_manifest = bitcode::encode(&manifest);
    let path = dest_dir.join(format!("{}.manifest", package));
    fs_utils::write(&path, encoded_manifest).await?;

    // Move wasm file to target directory
    let src = wasm_path;
    let dst = dest_dir.join(format!("{}.wasm", package));
    fs_utils::rename(src, dst).await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildType {
    Debug,
    Release,
    GenerateManifest,
}

async fn build_raw<E>(
    directory: PathBuf,
    packages: Vec<String>,
    build_type: BuildType,
) -> Result<(), E>
where
    E: rancor::Source,
{
    let mut command = CargoCommand::new("build");
    command
        .packages(packages.into_iter())
        .inner
        .current_dir(directory)
        .args(&["--target", "wasm32-unknown-unknown"])
        .env("RUSTFLAGS", "-C link-arg=--import-memory")
        .stderr(Stdio::piped());

    if build_type == BuildType::Release {
        command.inner.arg("--release");
    }

    let mut child = command
        .inner
        .spawn()
        .into_with_trace(|| format!("Could not start cargo"))?;

    // All human readable output for cargo is sent to stderr
    let stderr = child.stderr.take().unwrap();
    let stderr_handle = spawn(output_cargo_stderr(stderr));

    let (status, _) = (child.status(), stderr_handle).join().await;
    let status = status.into_error()?;
    if !status.success() {
        fail!(UnsuccessfulExitStatus {
            status: status.code()
        });
    }

    Ok(())
}

#[derive(Debug)]
struct UnsuccessfulExitStatus {
    status: Option<i32>,
}

impl std::fmt::Display for UnsuccessfulExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Exit status: {:?}", self.status)
    }
}

impl Error for UnsuccessfulExitStatus {}

async fn output_cargo_stderr(output: impl Read + Unpin) {
    let reader = BufReader::new(output);
    let mut lines = reader.lines();

    let mut err = false;
    while let Some(line) = lines.next().await {
        let line = line.expect("Failed to read line");
        err |= line.contains("error");
        if err {
            error!("{}", line);
        } else {
            info!("{}", line);
        }
    }
}

struct State {
    component_id_counter: u32,
    memory: wasmer::Memory,
    encoded_manifest: Option<Vec<u8>>,
}
type Context<'a> = wasmer::FunctionEnvMut<'a, State>;

fn reserve_component_id(mut context: Context) -> u32 {
    let state = context.data_mut();
    state.component_id_counter += 1;
    state.component_id_counter
}

fn submit_manifest(mut context: Context, manifest_ptr: u64) {
    let pointer: common::WasmPointer = manifest_ptr.into();

    let (data, store) = context.data_and_store_mut();

    let memory_view = data.memory.view(&store);
    let encoded = memory_view
        .copy_range_to_vec(pointer.into())
        .expect("failed to copy range");

    data.encoded_manifest = Some(encoded);
}

async fn wasm_export_encoded_manifest<E>(path: PathBuf) -> Result<Vec<u8>, E>
where
    E: rancor::Source,
{
    let bytes = fs_utils::read(&path).await?;

    // Create a Store.
    let mut store = wasmer::Store::default();

    // We then use our store and Wasm bytes to compile a `Module`.
    // A `Module` is a compiled WebAssembly module that isn't ready to execute yet.
    let module = wasmer::Module::new(&store, bytes).expect("invalid wasm");

    // Initiate shared memory pool
    let memory = wasmer::Memory::new(&mut store, wasmer::MemoryType::new(17, None, false))
        .expect("wasm memory allocation failed");

    let state = State {
        component_id_counter: 0,
        memory: memory.clone(),
        encoded_manifest: None,
    };
    let env = wasmer::FunctionEnv::new(&mut store, state);
    let import_object = wasmer::imports! {
        "bevy_harmonize" => {
            "reserve_component_id" => wasmer::Function::new_typed_with_env(&mut store, &env, reserve_component_id),
            "submit_manifest" => wasmer::Function::new_typed_with_env(&mut store, &env, submit_manifest),
        },
        "env" => {
            "memory" => memory,
        },
    };

    // We then use the `Module` and the import object to create an `Instance`.
    //
    // An `Instance` is a compiled WebAssembly module that has been set up
    // and is ready to execute.
    let instance = wasmer::Instance::new(&mut store, &module, &import_object)
        .expect("wasm instantiation failed");

    let init: wasmer::TypedFunction<(), ()> = instance
        .exports
        .get_typed_function(&store, "bevy_harmonize_generate_manifest")
        .expect("could not find bevy_harmonize_generate_manifest function");

    init.call(&mut store)
        .expect("failed to call bevy_harmonize_generate_manifest");

    let encoded = env
        .as_mut(&mut store)
        .encoded_manifest
        .take()
        .expect("mod never called submit_manifest");

    Ok(encoded)
}
