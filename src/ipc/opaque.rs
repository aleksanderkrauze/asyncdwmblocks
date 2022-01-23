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
//! ```no_run
//! use tokio::sync::{broadcast, mpsc};
//! use asyncdwmblocks::config::Config;
//! use asyncdwmblocks::statusbar::BlockRefreshMessage;
//! use asyncdwmblocks::ipc::{OpaqueServer, Server};
//!
//! # async fn doc() {
//! let (server_sender, mut server_receiver) = mpsc::channel(8);
//! let (_, termination_signal_receiver) = broadcast::channel::<()>(8);
//! let config = Config::default().arc();
//!
//! let mut server = OpaqueServer::new(server_sender, termination_signal_receiver, config);
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
//! ```no_run
//! use asyncdwmblocks::config::Config;
//! use asyncdwmblocks::statusbar::BlockRefreshMessage;
//! use asyncdwmblocks::block::BlockRunMode;
//! use asyncdwmblocks::ipc::{OpaqueNotifier, Notifier};
//!
//! # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
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
use tokio::sync::{broadcast, mpsc};

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
///
/// This enum doesn't implement `Clone`, because one of it's
/// servers (UdsServer) doesn't do it as well.
#[derive(Debug)]
pub enum OpaqueServer {
    /// TcpServer variant.
    #[cfg(feature = "tcp")]
    Tcp(tcp::TcpServer),
    /// UdsServer variant.
    #[cfg(feature = "uds")]
    UnixDomainSocket(uds::UdsServer),
}

impl OpaqueServer {
    /// Creates new `OpaqueServer` from configuration, sending half of a channel and
    /// a receiver for process termination by a signal.
    #[allow(unused_variables)] // In some features combination some input parameters won't be used
    pub fn new(
        sender: mpsc::Sender<BlockRefreshMessage>,
        termination_signal_receiver: broadcast::Receiver<()>,
        config: Arc<Config>,
    ) -> Self {
        let server_type = config.ipc.server_type;
        match server_type {
            #[cfg(feature = "tcp")]
            ServerType::Tcp => OpaqueServer::Tcp(tcp::TcpServer::new(sender, config)),
            #[cfg(feature = "uds")]
            ServerType::UnixDomainSocket => OpaqueServer::UnixDomainSocket(uds::UdsServer::new(
                sender,
                termination_signal_receiver,
                config,
            )),
        }
    }
}

#[async_trait]
impl Server for OpaqueServer {
    type Error = OpaqueServerError;

    async fn run(&mut self) -> Result<(), Self::Error> {
        match self {
            #[cfg(feature = "tcp")]
            Self::Tcp(server) => server.run().await.map_err(Self::Error::from),
            #[cfg(feature = "uds")]
            Self::UnixDomainSocket(server) => server.run().await.map_err(Self::Error::from),
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
            Self::Tcp(notifier) => notifier.push_message(message),
            #[cfg(feature = "uds")]
            Self::UnixDomainSocket(notifier) => notifier.push_message(message),
        }
    }

    async fn send_messages(self) -> Result<(), Self::Error> {
        match self {
            #[cfg(feature = "tcp")]
            Self::Tcp(notifier) => notifier.send_messages().await.map_err(Self::Error::from),
            #[cfg(feature = "uds")]
            Self::UnixDomainSocket(notifier) => {
                notifier.send_messages().await.map_err(Self::Error::from)
            }
        }
    }
}

#[cfg(test)]
#[allow(unused_imports)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;
    use crate::{
        block::BlockRunMode,
        config,
        ipc::frame::{Frame, Frames},
    };
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
    use tokio::sync::{broadcast, mpsc};

    macro_rules! opaque_server {
        ($config:expr, $stream_type:ty, $connect_value:expr) => {
            let (sender, mut receiver) = mpsc::channel(8);
            let (_, termination_signal_receiver) = broadcast::channel(8);
            let messages = vec![
                BlockRefreshMessage::new("block1".into(), BlockRunMode::Normal),
                BlockRefreshMessage::new("block2".into(), BlockRunMode::Button(1)),
                BlockRefreshMessage::new("block3".into(), BlockRunMode::Button(3)),
                BlockRefreshMessage::new("block4".into(), BlockRunMode::Button(4)),
            ];
            let expected_messages = messages.clone();

            let mut server =
                OpaqueServer::new(sender, termination_signal_receiver, Arc::clone(&$config));
            tokio::spawn(async move {
                let _ = server.run().await;
            });

            tokio::spawn(async move {
                let mut stream = <$stream_type>::connect($connect_value).await.unwrap();

                let frames: Frames = messages.into_iter().map(Frame::from).collect();
                let data = frames.encode();

                stream.write_all(data.as_slice()).await.unwrap();
            });

            assert_eq!(receiver.recv().await.unwrap(), expected_messages[0]);
            assert_eq!(receiver.recv().await.unwrap(), expected_messages[1]);
            assert_eq!(receiver.recv().await.unwrap(), expected_messages[2]);
            assert_eq!(receiver.recv().await.unwrap(), expected_messages[3]);
        };
    }

    macro_rules! opaque_notifier {
        ($config:expr, $listener:expr) => {
            let messages = vec![
                BlockRefreshMessage::new("block1".into(), BlockRunMode::Normal),
                BlockRefreshMessage::new("block2".into(), BlockRunMode::Button(1)),
                BlockRefreshMessage::new("block3".into(), BlockRunMode::Button(3)),
                BlockRefreshMessage::new("block4".into(), BlockRunMode::Button(4)),
            ];
            let expected_messages: Frames = messages.clone().into_iter().map(Frame::from).collect();

            let mut notifier = OpaqueNotifier::new(Arc::clone(&$config));
            tokio::spawn(async move {
                for message in messages {
                    notifier.push_message(message);
                }
                notifier.send_messages().await.unwrap();
            });

            let mut buff = Vec::new();
            let (mut stream, _) = $listener.accept().await.unwrap();
            stream.read_to_end(&mut buff).await.unwrap();
            let frames = Frames::from(buff.as_slice());

            assert_eq!(frames, expected_messages);
        };
    }

    #[cfg(feature = "tcp")]
    #[tokio::test]
    async fn opaque_server_tcp() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44010 },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        opaque_server!(
            config,
            TcpStream,
            (Ipv4Addr::LOCALHOST, config.ipc.tcp.port)
        );
    }

    #[cfg(feature = "uds")]
    #[tokio::test]
    async fn opaque_server_uds() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-opaque-server-uds-{}.socket",
            timestamp
        ));

        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        opaque_server!(config, UnixStream, &config.ipc.uds.addr);
    }

    #[tokio::test]
    #[cfg(feature = "tcp")]
    async fn opaque_notifier_tcp() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44011 },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        opaque_notifier!(
            config,
            TcpListener::bind((Ipv4Addr::LOCALHOST, config.ipc.tcp.port))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    #[cfg(feature = "uds")]
    async fn opaque_notifier_uds() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-opaque-notifier-uds-{}.socket",
            timestamp
        ));

        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        opaque_notifier!(config, UnixListener::bind(&config.ipc.uds.addr).unwrap());

        fs::remove_file(&config.ipc.uds.addr).unwrap();
    }
}
