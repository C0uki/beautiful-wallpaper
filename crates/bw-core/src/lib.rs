//! Portable core of the beautiful-wallpaper shell.
//!
//! Everything here is free of Tauri and of Win32, so it builds and its tests run
//! on any host. The Windows-specific half — window layering, the desktop
//! wallpaper COM interface, WASAPI, SMTC and the rest — lives in the shell crate.
//!
//! The split exists so the parts most likely to be wrong (the Material 3 colour
//! pipeline, the config schema, provider response parsing) can be tested without
//! a Windows machine in the loop.

pub mod ai;
pub mod booru;
pub mod brightness;
pub mod calc;
pub mod capture;
pub mod chat;
pub mod chrome;
pub mod config;
pub mod crosshair;
pub mod dock;
pub mod launcher;
pub mod menu;
pub mod notifications;
pub mod ocr;
pub mod overlay;
pub mod paths;
pub mod persistent;
pub mod search;
pub mod session;
pub mod settings;
pub mod shelf;
pub mod sysinfo;
pub mod theme;
pub mod todo;
pub mod wallpaper;

pub use config::Config;
pub use notifications::{NewNotification, Notification, Urgency};
pub use persistent::Persistent;
pub use theme::{GeneratedTheme, Mode};
pub use todo::TodoItem;
