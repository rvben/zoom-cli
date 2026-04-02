//! Test helpers shared across module tests.
//! Only compiled in test builds.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// Global mutex to serialize tests that mutate environment variables.
static PROCESS_ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct ProcessEnvLock(#[allow(dead_code)] MutexGuard<'static, ()>);

impl ProcessEnvLock {
    pub fn acquire() -> Result<Self, std::sync::PoisonError<MutexGuard<'static, ()>>> {
        Ok(Self(PROCESS_ENV_LOCK.lock()?))
    }
}

/// RAII guard that sets an env var and restores (or removes) it on drop.
pub struct EnvVarGuard {
    name: String,
    original: Option<String>,
}

impl EnvVarGuard {
    pub fn set(name: &str, value: &str) -> Self {
        let original = std::env::var(name).ok();
        // SAFETY: test-only helper, single-threaded via ProcessEnvLock
        unsafe { std::env::set_var(name, value) };
        Self { name: name.to_owned(), original }
    }

    pub fn unset(name: &str) -> Self {
        let original = std::env::var(name).ok();
        // SAFETY: test-only helper, single-threaded via ProcessEnvLock
        unsafe { std::env::remove_var(name) };
        Self { name: name.to_owned(), original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            // SAFETY: restoring previous state inside test teardown
            Some(v) => unsafe { std::env::set_var(&self.name, v) },
            None => unsafe { std::env::remove_var(&self.name) },
        }
    }
}

/// Set XDG_CONFIG_HOME to the given directory for the duration of the test.
pub fn set_config_dir_env(dir: &Path) -> EnvVarGuard {
    EnvVarGuard::set("XDG_CONFIG_HOME", &dir.to_string_lossy())
}

/// Write a config file to `<dir>/zoom-cli/config.toml`, creating parent dirs.
pub fn write_config(dir: &Path, content: &str) -> Result<PathBuf, std::io::Error> {
    let config_dir = dir.join("zoom-cli");
    std::fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("config.toml");
    std::fs::write(&path, content)?;
    Ok(path)
}
