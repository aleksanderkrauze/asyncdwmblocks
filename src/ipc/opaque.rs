//! This module allows to create "opaque" [Servers](Server) and [Notifiers](Notifier).
//!
//! When constructing Server/Notifier from [Config] it is easier to have one abstract
//! Server/Notifier and use it rather than matching on [ServerType], specifying
//! features and manually constructing each Server/Notifier. This module solves this
//! problem by defining [`OpaqueServer`] and [`OpaqueNotifier`] that act in aforementioned
//! way and implement themselves [Server] and [Notifier] traits respectively.
//!
//! Additionally there is [`OpaqueServerError`] and [`OpaqueNotifierError`] that abstract
//! over Server/Notifiers associated `Error` types. They internally hold `Box<dyn Error>`,
//! so the only thing you can do with them is to display them as errors.
//!
//! # Examples
//!
//! Example usage of `OpaqueServer`:
//!
//! ```
//! use tokio::sync::mpsc;
//! use asyncdwmblocks::config::Config;
//! use asyncdwmblocks::statusbar::BlockRefreshMessage;
//! use asyncdwmblocks::ipc::{OpaqueServer, Server};
//!
//! # #[tokio::main]
//! # async fn _main() {
//! let (server_sender, mut server_receiver) = mpsc::channel(8);
//! let config = Config::default().arc();
//!
//! let server = OpaqueServer::new(server_sender, config);
//!
//! tokio::spawn(async move {
//!     // Use server as a normal Server
//!     let _ = server.run().await;
//! });
//!
//! while let Some(msg) = server_receiver.recv().await {
//!     // process messages
//! }
//! # }
//! ```
//!
//! Example usage of `OpaqueNotifier`:
//!
//! ```
//! use asyncdwmblocks::config::Config;
//! use asyncdwmblocks::statusbar::BlockRefreshMessage;
//! use asyncdwmblocks::block::BlockRunMode;
//! use asyncdwmblocks::ipc::{OpaqueNotifier, Notifier};
//!
//! # #[tokio::main]
//! # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::default().arc();
//! let msg = BlockRefreshMessage { name: "block".to_string(), mode: BlockRunMode::Normal };
//!
//! // Use notifier as a normal Notifier
//! let mut notifier = OpaqueNotifier::new(config);
//!
//! notifier.push_message(msg);
//! notifier.send_messages().await?;
//! # Ok(())
//! # }
//! ```

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{Notifier, Server, ServerType};
use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

#[cfg(feature = "tcp")]
use super::tcp;
#[cfg(feature = "uds")]
use super::uds;

/// Abstraction over [Server::Error] associated type for [OpaqueServer].
#[derive(Debug)]
pub struct OpaqueServerError(Box<dyn Error + Send>);

impl fmt::Display for OpaqueServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for OpaqueServerError {}

#[cfg(feature = "tcp")]
impl From<tcp::server::TcpServerError> for OpaqueServerError {
    fn from(err: tcp::server::TcpServerError) -> Self {
        Self(Box::new(err))
    }
}

#[cfg(feature = "uds")]
impl From<uds::server::UdsServerError> for OpaqueServerError {
    fn from(err: uds::server::UdsServerError) -> Self {
        Self(Box::new(err))
    }
}

/// Abstraction over [Servers](Server).
#[derive(Debug, Clone)]
pub enum OpaqueServer {
    /// TcpServer variant.
    #[cfg(feature = "tcp")]
    Tcp(tcp::TcpServer),
    /// UdsServer variant.
    #[cfg(feature = "uds")]
    UnixDomainSocket(uds::UdsServer),
}

impl OpaqueServer {
    /// Creates new `OpaqueServer` from configuration and sending half of a channel.
    pub fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self {
        let server_type = config.ipc.server_type;
        match server_type {
            #[cfg(feature = "tcp")]
            ServerType::Tcp => OpaqueServer::Tcp(tcp::TcpServer::new(sender, config)),
            #[cfg(feature = "uds")]
            ServerType::UnixDomainSocket => {
                OpaqueServer::UnixDomainSocket(uds::UdsServer::new(sender, config))
            }
        }
    }
}

#[async_trait]
impl Server for OpaqueServer {
    type Error = OpaqueServerError;

    async fn run(&mut self) -> Result<(), Self::Error> {
        match self {
            #[cfg(feature = "tcp")]
            OpaqueServer::Tcp(server) => server.run().await.map_err(Self::Error::from),
            #[cfg(feature = "uds")]
            OpaqueServer::UnixDomainSocket(server) => server.run().await.map_err(Self::Error::from),
        }
    }
}

/// Abstraction over [Notifier::Error] associated type for [OpaqueNotifier].
#[derive(Debug)]
pub struct OpaqueNotifierError(Box<dyn Error>);

impl fmt::Display for OpaqueNotifierError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for OpaqueNotifierError {}

#[cfg(feature = "tcp")]
impl From<tcp::notifier::TcpNotifierError> for OpaqueNotifierError {
    fn from(err: tcp::notifier::TcpNotifierError) -> Self {
        Self(Box::new(err))
    }
}

#[cfg(feature = "uds")]
impl From<uds::notifier::UdsNotifierError> for OpaqueNotifierError {
    fn from(err: uds::notifier::UdsNotifierError) -> Self {
        Self(Box::new(err))
    }
}

/// Abstraction over [Notifiers](Notifier).
#[derive(Debug, PartialEq, Clone)]
pub enum OpaqueNotifier {
    /// TcpNotifier variant.
    #[cfg(feature = "tcp")]
    Tcp(tcp::TcpNotifier),
    /// UdsServer variant.
    #[cfg(feature = "uds")]
    UnixDomainSocket(uds::UdsNotifier),
}

impl OpaqueNotifier {
    /// Creates new `OpaqueNotifier` from configuration.
    pub fn new(config: Arc<Config>) -> Self {
        let server_type = config.ipc.server_type;
        match server_type {
            #[cfg(feature = "tcp")]
            ServerType::Tcp => OpaqueNotifier::Tcp(tcp::TcpNotifier::new(config)),
            #[cfg(feature = "uds")]
            ServerType::UnixDomainSocket => {
                OpaqueNotifier::UnixDomainSocket(uds::UdsNotifier::new(config))
            }
        }
    }
}

#[async_trait]
impl Notifier for OpaqueNotifier {
    type Error = OpaqueNotifierError;

    fn push_message(&mut self, message: BlockRefreshMessage) {
        match self {
            #[cfg(feature = "tcp")]
            OpaqueNotifier::Tcp(notifier) => notifier.push_message(message),
            #[cfg(feature = "uds")]
            OpaqueNotifier::UnixDomainSocket(notifier) => notifier.push_message(message),
        }
    }

    async fn send_messages(self) -> Result<(), Self::Error> {
        match self {
            #[cfg(feature = "tcp")]
            OpaqueNotifier::Tcp(notifier) => {
                notifier.send_messages().await.map_err(Self::Error::from)
            }
            #[cfg(feature = "uds")]
            OpaqueNotifier::UnixDomainSocket(notifier) => {
                notifier.send_messages().await.map_err(Self::Error::from)
            }
        }
    }
}
