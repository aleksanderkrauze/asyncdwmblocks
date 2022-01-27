//! This module defines TCP versions of [Server] and [Notifier].
//!
//! For more informations read documentations of [`TcpServer`] and [`TcpNotifier`].

pub mod notifier;
pub mod server;

pub use notifier::TcpNotifier;
pub use server::TcpServer;

use super::{frame, handle_server_stream, Notifier, Server};

#[cfg(test)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config::{self, Config};
    use crate::ipc::ServerType;
    use crate::statusbar::BlockRefreshMessage;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn server_and_notifier() {
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::Tcp,
                tcp: config::ConfigIpcTcp { port: 44005 },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let (sender, mut receiver) = mpsc::channel(8);
        let messages = vec![
            BlockRefreshMessage::new("block1".into(), BlockRunMode::Normal),
            BlockRefreshMessage::new("block2".into(), BlockRunMode::Button(1)),
            BlockRefreshMessage::new("block3".into(), BlockRunMode::Button(3)),
            BlockRefreshMessage::new("block4".into(), BlockRunMode::Button(4)),
        ];
        let expected_messages = messages.clone();

        let mut server = TcpServer::new(sender, Arc::clone(&config));
        tokio::spawn(async move {
            server.run().await.unwrap();
        });

        let mut notifier = TcpNotifier::new(Arc::clone(&config));
        tokio::spawn(async move {
            for message in messages {
                notifier.push_message(message);
            }
            notifier.send_messages().await.unwrap();
        });

        assert_eq!(receiver.recv().await.unwrap(), expected_messages[0]);
        assert_eq!(receiver.recv().await.unwrap(), expected_messages[1]);
        assert_eq!(receiver.recv().await.unwrap(), expected_messages[2]);
        assert_eq!(receiver.recv().await.unwrap(), expected_messages[3]);
    }
}
