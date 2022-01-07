use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;
use crate::server::Notifier;
use crate::statusbar::BlockRefreshMessage;

#[derive(Debug)]
pub enum TcpNotifierError {
    IO(std::io::Error),
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
        Ok(())
    }
}
