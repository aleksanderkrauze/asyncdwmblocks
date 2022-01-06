// mod frames;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::statusbar::BlockRefreshMessage;

#[async_trait]
pub(self) trait Listener {
    fn new(sender: mpsc::Sender<BlockRefreshMessage>) -> Self;
}
