//! This module defines [UdsNotifier] and it's Error.

use std::error::Error;
use std::fmt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use super::{
    frame::{Frame, Frames},
    Notifier,
};
use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

/// [TcpNotifier]'s error. Currently it's a wrapper around [std::io::Error].
#[derive(Debug)]
pub enum UdsNotifierError {
    /// IO error.
    IO(io::Error),
}

impl From<io::Error> for UdsNotifierError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for UdsNotifierError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            UdsNotifierError::IO(err) => {
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

impl Error for UdsNotifierError {}

/// A Unix domain socket Notifier.
#[derive(Debug, PartialEq, Clone)]
pub struct UdsNotifier {
    config: Arc<Config>,
    buff: Vec<BlockRefreshMessage>,
}

impl UdsNotifier {
    /// Create a new notifier.
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            buff: Vec::new(),
        }
    }
}

#[async_trait]
impl Notifier for UdsNotifier {
    type Error = UdsNotifierError;

    fn push_message(&mut self, message: BlockRefreshMessage) {
        self.buff.push(message)
    }

    async fn send_messages(self) -> Result<(), Self::Error> {
        let mut stream = UnixStream::connect(&self.config.ipc.uds.addr).await?;

        let frames: Frames = self.buff.into_iter().map(Frame::from).collect();
        let data = frames.encode();

        stream.write_all(data.as_slice()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config;
    use crate::ipc::ServerType;
    use chrono::{DateTime, Utc};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tokio::fs;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn send_notification() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-notifier-{}.socket",
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

        let mut notifier = UdsNotifier::new(Arc::clone(&config));
        tokio::spawn(async move {
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
        let listener = UnixListener::bind(&config.ipc.uds.addr).unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        stream.read_to_end(&mut buff).await.unwrap();

        fs::remove_file(&config.ipc.uds.addr).await.unwrap();

        assert_eq!(
            buff.as_slice(),
            b"REFRESH cpu\r\nBUTTON 3 memory\r\nBUTTON 1 battery\r\n"
        );
    }
}
