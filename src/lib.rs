//! # leetrs
//!
//! Core library for the `leetrs-helix` CLI — a terminal-first LeetCode client.
//!
//! The crate is split into focused modules:
//! - [`auth`] — credential storage and browser cookie extraction
//! - [`client`] — authenticated HTTP/GraphQL client for the LeetCode API
//! - [`models`] — request/response data types
//! - [`picker`] — problem fetching, file generation, submission, and local caching
//! - [`tui`] — ratatui-based interactive problem browser
//! - [`config`] — TOML user config
use std::path::PathBuf;

pub mod error;

pub mod auth;

pub mod cache;

pub mod commands;

pub mod format;

pub mod log;

pub mod models;

pub mod client;

pub mod picker;

pub mod services;

pub mod tui;

pub mod config;

pub mod theme;

/// Old config directory used by upstream `leetrs`.
fn old_config_path() -> PathBuf {
    directories::BaseDirs::new()
        .expect("Failed to find directories")
        .home_dir()
        .join(".config/leetrs")
}

/// Returns the path to `~/.config/leetrs-helix/config.toml`, creating the directory if needed.
pub fn get_config_file() -> PathBuf {
    get_config_path().join("config.toml")
}

/// Returns `~/.config/leetrs-helix/`, creating it on first use.
///
/// If an old `~/.config/leetrs/` directory exists and the new one does not,
/// the entire directory is renamed so existing users keep their credentials
/// and config without manual migration.
pub fn get_config_path() -> PathBuf {
    let path = directories::BaseDirs::new()
        .expect("Failed to find directories")
        .home_dir()
        .join(".config/leetrs-helix");

    let old = old_config_path();
    if !path.exists() && old.exists() {
        if let Err(e) = std::fs::rename(&old, &path) {
            eprintln!("Failed to migrate config directory: {}", e);
        } else {
            println!("Migrated config directory: {:?} -> {:?}", old, path);
        }
    }

    if let Err(e) = std::fs::create_dir_all(&path) {
        eprintln!("Failed to create config directory: {}", e);
    }

    path
}
