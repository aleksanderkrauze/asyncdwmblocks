//! This module defines [TcpNotifier] and it's Error.

use std::error::Error;
use std::fmt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::{
    frame::{Frame, Frames},
    Notifier,
};
use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

/// [TcpNotifier]'s error. Currently it's a wrapper around [std::io::Error].
#[derive(Debug)]
pub enum TcpNotifierError {
    /// IO error.
    IO(io::Error),
}

impl From<io::Error> for TcpNotifierError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for TcpNotifierError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::IO(err) => {
                let mut msg = format!("io error: {}", err);

                if err.kind() == io::ErrorKind::ConnectionRefused {
                    msg.push_str("\nCheck if you are running asyncdwmblocks.");
                }

                msg
            }
        };

        write!(f, "{}", msg)
    }
}

impl Error for TcpNotifierError {}

#[cfg(test)]
impl TcpNotifierError {
    pub(crate) fn into_io_error(self) -> Option<io::Error> {
        #[allow(unreachable_patterns)]
        match self {
            Self::IO(error) => Some(error),
            _ => None,
        }
    }
}

/// A TCP notifier.
///
/// This notifier collects messages ([`BlockRefreshMessage`]) and then
/// connects to TCP socket on *localhost* and port defined in
/// [config](crate::config::ConfigIpcTcp::port)
/// and sends encoded messages to a listening server.
#[derive(Debug, PartialEq, Clone)]
pub struct TcpNotifier {
    config: Arc<Config>,
    buff: Vec<BlockRefreshMessage>,
}

impl TcpNotifier {
    /// Create a new notifier.
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            buff: Vec::new(),
        }
    }
}

#[async_trait]
impl Notifier for TcpNotifier {
    type Error = TcpNotifierError;

    fn push_message(&mut self, message: BlockRefreshMessage) {
        self.buff.push(message)
    }

    async fn send_messages(self) -> Result<(), Self::Error> {
        let mut stream =
            TcpStream::connect((Ipv4Addr::LOCALHOST, self.config.ipc.tcp.port)).await?;

        let frames: Frames = self.buff.into_iter().map(Frame::from).collect();
        let data = frames.encode();

        stream.write_all(data.as_slice()).await?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config;
    use crate::ipc::ServerType;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_notification() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44001 },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let config_notifier = Arc::clone(&config);
        tokio::spawn(async move {
            let mut notifier = TcpNotifier::new(config_notifier);
            notifier.push_message(BlockRefreshMessage::new(
                String::from("cpu"),
                BlockRunMode::Normal,
            ));
            notifier.push_message(BlockRefreshMessage::new(
                String::from("memory"),
                BlockRunMode::Button(3),
            ));
            notifier.push_message(BlockRefreshMessage::new(
                String::from("battery"),
                BlockRunMode::Button(1),
            ));
            notifier.send_messages().await.unwrap();
        });

        let mut buff = Vec::new();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.ipc.tcp.port))
            .await
            .unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        stream.read_to_end(&mut buff).await.unwrap();

        assert_eq!(
            buff.as_slice(),
            b"REFRESH cpu\r\nBUTTON 3 memory\r\nBUTTON 1 battery\r\n"
        );
    }

    #[tokio::test]
    async fn notification_connection_error() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44006 },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let mut notifier = TcpNotifier::new(config);
        notifier.push_message(BlockRefreshMessage::new(
            String::from("block"),
            BlockRunMode::Normal,
        ));
        let n = notifier.send_messages().await;

        assert!(n.is_err());
        assert_eq!(
            n.unwrap_err().into_io_error().unwrap().kind(),
            io::ErrorKind::ConnectionRefused
        );
    }
}
