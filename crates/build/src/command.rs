use anyhow::*;
use async_process::Command;
use async_process::Stdio;
use async_std::{
    io::{prelude::BufReadExt, BufReader, Read},
    stream::StreamExt,
    task::spawn,
};
use bevy_utils::tracing::warn;
use bevy_utils::tracing::{error, info};
use futures_concurrency::prelude::*;
use std::path::Path;
use std::{ffi::OsStr, str};

pub struct CargoCommand {
    inner: Command,
}

impl CargoCommand {
    pub fn new(kind: &str) -> Result<Self> {
        let program = which::which("cargo")?;
        let mut command = Command::new(program);
        command.arg(kind);
        Ok(Self { inner: command })
    }

    pub fn packages<I, S>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for name in names {
            self.inner.arg("-p");
            self.inner.arg(name.as_ref());
        }
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn target<S: AsRef<OsStr>>(&mut self, target: S) -> &mut Self {
        self.inner.arg("--target");
        self.inner.arg(target);
        self
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, value);
        self
    }

    pub async fn spawn(&mut self) -> Result<()> {
        let mut child = self
            .inner
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!(e))
            .with_context(|| format!("Could not start cargo"))?;

        // All human readable output for cargo is sent to stderr
        let stdout = child.stdout.take().unwrap();
        let stdout_handle = spawn(output_cargo_std(stdout, true));
        let stderr = child.stderr.take().unwrap();
        let stderr_handle = spawn(output_cargo_std(stderr, true));

        let (status, _, _) = (child.status(), stdout_handle, stderr_handle).join().await;
        let status = status?;
        if !status.success() {
            bail!("Cargo build failed with status: {}", status);
        }

        Ok(())
    }
}

enum Level {
    Info,
    Warn,
    Error,
}

async fn output_cargo_std(output: impl Read + Unpin, error: bool) {
    let reader = BufReader::new(output);
    let mut lines = reader.lines();

    let mut level = Level::Info;
    while let Some(line) = lines.next().await {
        let line = line.expect("Failed to read line");
        if line.starts_with("error") {
            level = Level::Error;
        } else if line.starts_with("warning") {
            level = Level::Warn;
        }
        match level {
            Level::Info => info!("{}", line),
            Level::Warn => warn!("{}", line),
            Level::Error => error!("{}", line),
        }
        if line.is_empty() {
            level = Level::Info;
        }
    }
}
