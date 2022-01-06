mod tcp;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

#[async_trait]
pub(self) trait Listener {
    fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self;
    async fn run(&mut self);
}

pub enum ListenerTypes {
    Tcp,
}
