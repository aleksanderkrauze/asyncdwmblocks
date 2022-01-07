use std::collections::VecDeque;
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

#[derive(Debug)]
pub enum TcpNotifierError {
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

pub struct TcpNotifier {
    config: Arc<Config>,
    buff: VecDeque<BlockRefreshMessage>,
}

#[async_trait]
impl Notifier for TcpNotifier {
    type Error = TcpNotifierError;

    fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            buff: VecDeque::new(),
        }
    }

    fn push_message(&mut self, message: BlockRefreshMessage) {
        self.buff.push_back(message)
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
        let config = Arc::new(Config {
            tcp_port: 44001,
            ..Config::default()
        });

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
