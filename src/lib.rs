//! # Features
//!
//! - `tcp`: Enables IPC through tcp sockets.
//! - `ipc`: Builds library with support of IPC (inter process communication).
//! This is automatically enabled when needed and should not be manually selected.

pub mod block;
pub mod config;
#[cfg(feature = "ipc")]
pub mod ipc;
pub mod statusbar;
pub mod utils;
pub mod x11;
