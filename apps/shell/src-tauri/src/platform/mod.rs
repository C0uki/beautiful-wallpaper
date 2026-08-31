//! Everything that only exists on Windows.
//!
//! Gated so the rest of the crate can be read — and reasoned about — without
//! `#[cfg]` noise. On any other host these modules are simply absent, and the
//! crate is only ever built for Windows.

#[cfg(windows)]
pub mod appicon;
#[cfg(windows)]
pub mod apps;
#[cfg(windows)]
pub mod audio;
#[cfg(windows)]
pub mod brightness;
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod deskclick;
#[cfg(windows)]
pub mod dragout;
#[cfg(windows)]
pub mod launch;
#[cfg(windows)]
pub mod mixer;
#[cfg(windows)]
pub mod ocr;
#[cfg(windows)]
pub mod power;
#[cfg(windows)]
pub mod radios;
#[cfg(windows)]
pub mod session;
#[cfg(windows)]
pub mod tray;
#[cfg(windows)]
pub mod wallpaper;
#[cfg(windows)]
pub mod win;
#[cfg(windows)]
pub mod windows;
