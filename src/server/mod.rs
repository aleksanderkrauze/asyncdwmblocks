pub mod frame;
pub mod tcp;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

#[async_trait]
pub trait Listener {
    type Error: std::error::Error;
    fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self;
    async fn run(&self) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ServerType {
    Tcp,
}

pub fn get_listener(
    listener_type: ServerType,
    sender: mpsc::Sender<BlockRefreshMessage>,
    config: Arc<Config>,
) -> impl Listener {
    match listener_type {
        ServerType::Tcp => tcp::TcpListener::new(sender, config),
    }
}

pub use tcp::TcpListener;
