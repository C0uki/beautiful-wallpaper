//! Everything that only exists on Windows.
//!
//! Gated so the rest of the crate can be read — and reasoned about — without
//! `#[cfg]` noise. On any other host these modules are simply absent, and the
//! crate is only ever built for Windows.

#[cfg(windows)]
pub mod tray;
#[cfg(windows)]
pub mod wallpaper;
#[cfg(windows)]
pub mod win;
