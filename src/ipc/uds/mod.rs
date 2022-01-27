//! This module defines Unix domain socket versions of [Server] and [Notifier].
//!
//! For more informations read documentations of [`UdsServer`] and [`UdsNotifier`].

pub mod notifier;
pub mod server;

pub use notifier::UdsNotifier;
pub use server::UdsServer;

use super::{frame, handle_server_stream, Notifier, Server};

#[cfg(test)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;
    use crate::block::BlockRunMode;
    use crate::config::{self, Config};
    use crate::ipc::ServerType;
    use crate::statusbar::BlockRefreshMessage;
    use chrono::{DateTime, Utc};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::SystemTime;
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn server_and_notifier() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-server-and-notifier-{}.socket",
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

        let (sender, mut receiver) = mpsc::channel(8);
        let (_, termination_signal_receiver) = broadcast::channel(8);
        let messages = vec![
            BlockRefreshMessage::new("block1".into(), BlockRunMode::Normal),
            BlockRefreshMessage::new("block2".into(), BlockRunMode::Button(1)),
            BlockRefreshMessage::new("block3".into(), BlockRunMode::Button(3)),
            BlockRefreshMessage::new("block4".into(), BlockRunMode::Button(4)),
        ];
        let expected_messages = messages.clone();

        let mut server = UdsServer::new(sender, termination_signal_receiver, Arc::clone(&config));
        tokio::spawn(async move {
            server.run().await.unwrap();
        });

        let mut notifier = UdsNotifier::new(Arc::clone(&config));
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
