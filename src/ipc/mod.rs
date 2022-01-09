//! This module enablas IPC (inter process communication).
//!
//! This module defines two different (struct) types.
//! A Server (created by implementing [`Server`] trait) and
//! Notifier (created by implementing [`Notifier`] trait).
//! Server is used to listen (through network, pipe, bus or any
//! other IPC method) for requests on refresh blocks and then passes
//! them out through a channel. Notifier is on the other hand used
//! to sent those requests (from another processes).
//!
//! Sent messages are streams of bytes. Translation between them
//! and (in this case) [`BlockRefreshMessage`] and vice versa is
//! performed by [`Frames`](frame::Frames) in the [frame] module.
//!
//! [`ServerType`] is used in [`Config`] to select which server
//! (and notifier) type should be used in binaries. Two helper functions:
//! [get_server] and [get_notifier] can be used to get server and notifier
//! from config.

pub mod frame;
#[cfg(feature = "tcp")]
pub mod tcp;

use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

/// This trait defines public API for servers.
#[async_trait]
pub trait Server {
    /// Server error type (returned from running server).
    type Error: Error;
    /// Creates a new server.
    ///
    /// **sender** is a sender half of the channel used to
    /// communicate that some request was made.
    fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self;
    /// Start running server loop.
    async fn run(&self) -> Result<(), Self::Error>;
}

/// This trait defines public API for notifiers.
///
/// Notifiers store messages in queue by calling
/// [push_message](Notifier::push_message) and then send them all
/// at once by calling [send_messages](Notifier::send_messages).
#[async_trait]
pub trait Notifier {
    /// Notifier error type.
    type Error: Error;
    /// Create a new notifier.
    fn new(config: Arc<Config>) -> Self;
    /// Add message for sending.
    fn push_message(&mut self, message: BlockRefreshMessage);
    /// Send all stored messages.
    ///
    /// This method consumes notifier, because it is no longer needed.
    /// All messages should be batched together to avoid opening
    /// connections multiple times.
    async fn send_messages(self) -> Result<(), Self::Error>;
}

/// Type of server and notifier.
///
/// This enum (used in [`Config`]) specifies which method
/// of IPC should be used by binaries and is used by functions
/// [get_server] and [get_notifier] to create according objects.
#[derive(Debug, PartialEq, Clone)]
pub enum ServerType {
    /// Communicate through TCP socket.
    ///
    /// Port is defined in [`Config`].
    #[cfg(feature = "tcp")]
    Tcp,
}

/// Creates server from configuration.
pub fn get_server(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> impl Server {
    let server_type = config.server_type.clone();
    match server_type {
        #[cfg(feature = "tcp")]
        ServerType::Tcp => tcp::TcpServer::new(sender, config),
    }
}

/// Creates notifier from configuration.
pub fn get_notifier(config: Arc<Config>) -> impl Notifier {
    let server_type = config.server_type.clone();
    match server_type {
        #[cfg(feature = "tcp")]
        ServerType::Tcp => tcp::TcpNotifier::new(config),
    }
}
