//! Centralised disk-cache service for the leetrs engine.
//!
//! All paths under the OS data directory are owned by [`CacheService`].
//! This eliminates the duplicated `ProjectDirs` calls that were scattered
//! across `picker.rs` and `auth.rs`.
use crate::error::{EngineError, Result};
use directories::ProjectDirs;
use serde::{Serialize, de::DeserializeOwned};
use std::{fs, path::PathBuf};

/// Provides typed read/write access to the application's OS data directory.
///
/// All cache files live under the directory returned by
/// `ProjectDirs::from("com", "shadowmkj", "leetrs").data_dir()`.
pub struct CacheService {
    data_dir: PathBuf,
}

impl CacheService {
    /// Creates a new [`CacheService`], initialising the data directory on disk
    /// if it does not already exist.
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "shadowmkj", "leetrs-helix")
            .expect("Failed to resolve OS data directory");
        let data_dir = project_dirs.data_dir().to_path_buf();
        if let Err(e) = fs::create_dir_all(&data_dir) {
            eprintln!("❌ Failed to create data directory: {}", e);
        }
        Self { data_dir }
    }

    /// Returns the full path for a named cache file inside the data directory.
    pub fn path(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }

    /// Returns the path to `data.json` (the cached problem list).
    pub fn problems_path(&self) -> PathBuf {
        self.path("data.json")
    }

    /// Returns the path to `user.json` (the cached user profile).
    pub fn user_path(&self) -> PathBuf {
        self.path("user.json")
    }

    /// Deserialises a typed value from a named cache file.
    ///
    /// Returns `Ok(None)` if the file does not exist, `Err` on I/O or parse failure.
    pub fn read<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        let path = self.path(name);
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let value: T = serde_json::from_str(&contents).map_err(EngineError::from)?;
                Ok(Some(value))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(EngineError::Io(e)),
        }
    }

    /// Serialises a value and writes it to a named cache file.
    pub fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
        let path = self.path(name);
        let contents = serde_json::to_string(value).map_err(EngineError::from)?;
        fs::write(&path, &contents).map_err(EngineError::Io)
    }

    /// Reads raw string content from a named cache file.
    ///
    /// Returns `None` if the file does not exist.
    pub fn read_raw(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.path(name)).ok()
    }

    /// Writes raw string content to a named cache file.
    pub fn write_raw(&self, name: &str, content: &str) -> Result<()> {
        fs::write(self.path(name), content).map_err(EngineError::Io)
    }
}

impl Default for CacheService {
    fn default() -> Self {
        Self::new()
    }
}
