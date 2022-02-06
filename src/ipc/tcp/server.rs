//! This module defines [TcpServer] and it's Error.

use std::error::Error;
use std::fmt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Sender};

use super::{handle_server_stream, Server};
use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

/// [TcpServer]'s error. Currently it's a wrapper around [std::io::Error].
#[derive(Debug)]
pub enum TcpServerError {
    /// IO Error.
    IO(io::Error),
}

impl From<io::Error> for TcpServerError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for TcpServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg: String = match self {
            Self::IO(err) => {
                let mut msg = format!("io error: {}", err);

                if err.kind() == io::ErrorKind::AddrInUse {
                    msg.push_str("\nCheck if anther program is using it, or if another instance of asyncdwmblocks is already running.");
                }

                msg
            }
        };

        write!(f, "{}", msg)
    }
}

impl Error for TcpServerError {}

#[cfg(test)]
impl TcpServerError {
    pub(crate) fn into_io_error(self) -> Option<io::Error> {
        #[allow(unreachable_patterns)]
        match self {
            Self::IO(error) => Some(error),
            _ => None,
        }
    }
}

/// A TCP server.
///
/// This server will listen to TCP connections on *localhost*
/// and port defined in [config](crate::config::ConfigIpcTcp::port).
/// It will run until receiving half of **sender** channel is
/// closed or accepting new connection fails.
#[derive(Debug, Clone)]
pub struct TcpServer {
    config: Arc<Config>,
    sender: Sender<BlockRefreshMessage>,
}

impl TcpServer {
    /// Creates new TCP server.
    ///
    /// **sender** is a sender half of the channel used to
    /// communicate that some request was made.
    pub fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self {
        Self { sender, config }
    }
}

#[async_trait]
impl Server for TcpServer {
    type Error = TcpServerError;

    async fn run(&mut self) -> Result<(), Self::Error> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, self.config.ipc.tcp.port)).await?;

        let (cancelation_sender, mut cancelation_receiver) = mpsc::channel::<()>(1);
        loop {
            let stream = tokio::select! {
                accepted_stream = listener.accept() => {
                    let (stream, _) = accepted_stream?;
                    stream
                }
                _ = cancelation_receiver.recv() => break
            };

            let cancelation_sender = cancelation_sender.clone();
            let message_sender = self.sender.clone();
            tokio::spawn(async move {
                handle_server_stream(stream, message_sender, cancelation_sender).await;
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config;
    use crate::ipc::ServerType;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::sync::mpsc::channel;
    use tokio::time;

    #[tokio::test]
    async fn run_tcp_server() {
        let (sender, mut receiver) = channel(8);
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44002 },
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();

        let mut server = TcpServer::new(sender, Arc::clone(&config));
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        tokio::spawn(async move {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, config.ipc.tcp.port))
                .await
                .unwrap();

            stream
                .write_all(b"REFRESH date\r\nBUTTON 3 weather\r\n")
                .await
                .unwrap();
        });

        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("date"), BlockRunMode::Normal)
        );
        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("weather"), BlockRunMode::Button(3))
        );
    }

    #[tokio::test]
    async fn tcp_server_binding_error() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44004 },
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();

        let (sender1, _) = mpsc::channel(8);
        let (sender2, _) = mpsc::channel(8);

        let mut server1 = TcpServer::new(sender1, Arc::clone(&config));
        tokio::spawn(async move {
            let _ = server1.run().await;
        });

        time::sleep(time::Duration::from_millis(100)).await;

        let mut server2 = TcpServer::new(sender2, Arc::clone(&config));
        let s = server2.run().await;

        assert!(s.is_err());
        assert_eq!(
            s.unwrap_err().into_io_error().unwrap().kind(),
            io::ErrorKind::AddrInUse
        );
    }
}
