//! This module defines TCP versions of [Server](super::Server)
//! and [Notifier](super::Notifier).
//!
//! For more informations read documentations of
//! [`TcpServer`] and [`TcpNotifier`].

pub mod notifier;
pub mod server;

pub use notifier::TcpNotifier;
pub use server::TcpServer;
