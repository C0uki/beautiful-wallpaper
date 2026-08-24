//! The beautiful-wallpaper shell.
//!
//! Portable logic — the config schema, the Material 3 pipeline, wallpaper
//! indexing — lives in `bw-core` so it can be tested anywhere. This crate is the
//! Windows half: window layering, the desktop wallpaper, system providers, IPC
//! and the Tauri surfaces that host the UI.

pub mod cli;
pub mod commands;
pub mod platform;
pub mod providers;
pub mod services;
pub mod state;
pub mod surfaces;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
