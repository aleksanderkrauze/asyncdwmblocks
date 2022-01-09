//! # Features
//!
//! - `tcp`: Enables IPC through tcp sockets.

pub mod block;
pub mod config;
#[cfg(feature = "ipc")]
pub mod ipc;
pub mod statusbar;
pub mod utils;
pub mod x11;
