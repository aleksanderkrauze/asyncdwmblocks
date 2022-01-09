//! # Features
//!
//! Internal:
//! - `ipc`: Builds library with support of IPC (inter process communication).
//! This is automatically enabled when needed and should not be manually selected.
//!
//! User selectable:
//! - `tcp`: Enables IPC through tcp sockets.
//! - `config-file`: Enables loading configuration from file. If not present, then
//! configuration will be created from source code.
//!
//! By default following features are enabled: `tcp`, `config-file`.

pub mod block;
pub mod config;
#[cfg(feature = "ipc")]
pub mod ipc;
pub mod statusbar;
pub mod utils;
pub mod x11;
