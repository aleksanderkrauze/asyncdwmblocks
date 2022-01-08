//! This module defines [TcpNotifier] and it's Error.

use std::error::Error;
use std::fmt;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::config::Config;
use crate::server::{
    frame::{Frame, Frames},
    Notifier,
};
use crate::statusbar::BlockRefreshMessage;

/// [TcpNotifier]'s error. Currently it's a wrapper around [std::io::Error].
#[derive(Debug)]
pub enum TcpNotifierError {
    /// IO error.
    IO(std::io::Error),
}

impl From<std::io::Error> for TcpNotifierError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for TcpNotifierError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            TcpNotifierError::IO(err) => format!("io error: {}", err),
        };

        write!(f, "{}", msg)
    }
}

impl Error for TcpNotifierError {}

/// A TCP notifier.
///
/// This notifier collects messages ([`BlockRefreshMessage`]) and then
/// connects to TCP socket on *localhost* and port defined in [Config::tcp_port]
/// and sends encoded messages to a listening server.
///
/// # Example
///
/// ```
/// use asyncdwmblocks::config::Config;
/// use asyncdwmblocks::server::{Notifier, tcp::TcpNotifier};
/// use asyncdwmblocks::block::BlockRunMode;
/// use asyncdwmblocks::statusbar::BlockRefreshMessage;
///
/// # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut notifier = TcpNotifier::new(Config::default().arc());
///
/// let messages = vec![
///     BlockRefreshMessage::new(String::from("battery"), BlockRunMode::Normal),
///     BlockRefreshMessage::new(String::from("backlight"), BlockRunMode::Button(3))
/// ];
/// for message in messages {
///     notifier.push_message(message);
/// }
///
/// notifier.send_messages().await?;
/// # Ok(())
/// # }
/// ```
pub struct TcpNotifier {
    config: Arc<Config>,
    buff: Vec<BlockRefreshMessage>,
}

#[async_trait]
impl Notifier for TcpNotifier {
    type Error = TcpNotifierError;

    fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            buff: Vec::new(),
        }
    }

    fn push_message(&mut self, message: BlockRefreshMessage) {
        self.buff.push(message)
    }

    async fn send_messages(self) -> Result<(), Self::Error> {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, self.config.tcp_port)).await?;

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
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_notification() {
        let config = Config {
            tcp_port: 44001,
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
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.tcp_port))
            .await
            .unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        stream.read_to_end(&mut buff).await.unwrap();

        assert_eq!(
            buff.as_slice(),
            b"REFRESH cpu\r\nBUTTON 3 memory\r\nBUTTON 1 battery\r\n"
        );
    }
}
