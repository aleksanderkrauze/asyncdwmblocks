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
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config;
    use crate::ipc::ServerType;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[ignore]
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

        todo!()
    }
}
