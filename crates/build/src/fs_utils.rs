use anyhow::*;
use async_fs;
use futures_lite::stream::StreamExt;
use std::path::{Path, PathBuf};

pub async fn read_dir<P>(path: P) -> Result<async_fs::ReadDir>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    async_fs::read_dir(path)
        .await
        .with_context(|| format!("Failed to read dir: {:?}", path))
}

pub async fn remove_dir_all<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    async_fs::remove_dir_all(path)
        .await
        .with_context(|| format!("Failed to remove dir: {:?}", path))
}

pub async fn remove_file<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    async_fs::remove_file(path)
        .await
        .with_context(|| format!("Failed to remove file: {:?}", path))
}

pub async fn create_dir_all<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if let Err(e) = async_fs::create_dir_all(path).await {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e).with_context(|| format!("Failed to create dir: {:?}", path));
        }
    }
    Ok(())
}

pub async fn rename<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let from = from.as_ref();
    let to = to.as_ref();
    async_fs::rename(from, to)
        .await
        .with_context(|| format!("Failed to rename file: {:?} -> {:?}", from, to))
}

pub async fn read<P>(path: P) -> Result<Vec<u8>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    async_fs::read(path)
        .await
        .with_context(|| format!("Failed to read file: {:?}", path))
}

pub async fn write<P, C>(path: P, contents: C) -> Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    let path = path.as_ref();
    async_fs::write(path, contents)
        .await
        .with_context(|| format!("Failed to write to file: {:?}", path))
}

pub async fn write_template<P, T>(path: P, template: T) -> Result<()>
where
    P: AsRef<Path>,
    T: std::fmt::Display,
{
    let path = path.as_ref();
    if let Some(directory) = path.parent() {
        create_dir_all(&directory).await?;
    }

    let contents = format!("{}", &template);
    write(path, contents).await?;
    Ok(())
}

/// Iterates through a directory's descendents, deleting those for whom the condition yields true
pub async fn empty_dir_conditional<P, C>(path: P, condition: C) -> Result<()>
where
    P: AsRef<Path>,
    C: Fn(&Path) -> bool,
{
    let mut entries = read_dir(path).await?;

    while let Some(entry) = entries.try_next().await? {
        let path = entry.path();
        if condition(&path) {
            if entry.file_type().await?.is_dir() {
                remove_dir_all(path).await?;
            } else {
                remove_file(path).await?;
            }
        }
    }
    Ok(())
}

pub async fn list_files_in_dir<P>(path: P) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut files = Vec::new();
    let mut dirs = vec![path.as_ref().to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let mut entries = read_dir(&dir).await?;
        while let Some(entry) = entries.try_next().await? {
            let path = entry.path();
            if entry.file_type().await?.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }

    Ok(files)
}
