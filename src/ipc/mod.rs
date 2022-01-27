//! This module enablas IPC (inter process communication).
//!
//! This module defines two different (struct) types.
//! A Server (created by implementing [`Server`] trait) and
//! Notifier (created by implementing [`Notifier`] trait).
//! Server is used to listen (through network, pipe, bus or any
//! other IPC method) for requests on refresh blocks and then passes
//! them out through a channel. Notifier is on the other hand used
//! to sent those requests (from another processes).
//!
//! Sent messages are streams of bytes. Translation between them
//! and (in this case) [`BlockRefreshMessage`] and vice versa is
//! performed by [`Frames`](frame::Frames) in the [frame] module.
//!
//! [`ServerType`] is used in [`Config`](crate::config::Config)
//! to select which server (and notifier) type should be used in binaries.
pub mod frame;
pub mod opaque;

#[cfg(feature = "tcp")]
pub mod tcp;
#[cfg(feature = "uds")]
pub mod uds;

use std::error::Error;
use std::fmt;

use async_trait::async_trait;
#[cfg(feature = "config-file")]
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;

use crate::statusbar::BlockRefreshMessage;
use frame::{Frame, Frames};

pub use opaque::{OpaqueNotifier, OpaqueServer};

/// This trait defines public API for servers.
#[async_trait]
pub trait Server {
    /// Server error type (returned from running server).
    type Error: Error + Send;

    /// Start running server loop.
    async fn run(&mut self) -> Result<(), Self::Error>;
}

/// This trait defines public API for notifiers.
///
/// Notifiers store messages in queue by calling
/// [push_message](Notifier::push_message) and then send them all
/// at once by calling [send_messages](Notifier::send_messages).
#[async_trait]
pub trait Notifier {
    /// Notifier error type.
    type Error: Error;

    /// Add message for sending.
    fn push_message(&mut self, message: BlockRefreshMessage);

    /// Send all stored messages.
    ///
    /// This method consumes notifier, because it is no longer needed.
    /// All messages should be batched together to avoid opening
    /// connections multiple times.
    async fn send_messages(self) -> Result<(), Self::Error>;
}

/// Type of server and notifier.
///
/// This enum (used in [`Config`](crate::config::Config))
/// specifies which method of IPC should be used by binaries
/// and is used by [OpaqueServer] and [OpaqueNotifier]
/// to create new servers/notifiers.
#[derive(Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
pub enum ServerType {
    /// Communicate through TCP socket.
    ///
    /// Port is defined in [`Config`](crate::config::Config).
    #[cfg(feature = "tcp")]
    #[cfg_attr(feature = "config-file", serde(rename = "tcp"))]
    Tcp,
    /// Communicate through Unix domain socket.
    ///
    /// Address is defined in [`Config`](crate::config::Config).
    #[cfg(feature = "uds")]
    #[cfg_attr(feature = "config-file", serde(rename = "uds"))]
    UnixDomainSocket,
}

impl fmt::Display for ServerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            #[cfg(feature = "tcp")]
            Self::Tcp => "TCP",
            #[cfg(feature = "uds")]
            Self::UnixDomainSocket => "Unix domain socket",
        };

        write!(f, "{}", msg)
    }
}

/// Universal (for `Server`s method to handle streams).
async fn handle_server_stream<S: AsyncRead + Unpin>(
    mut stream: S,
    message_sender: mpsc::Sender<BlockRefreshMessage>,
    cancelation_sender: mpsc::Sender<()>,
) {
    let mut buffer = [0u8; 1024];
    let nbytes = match stream.read(&mut buffer).await {
        Ok(n) => {
            if n == 0 {
                // Don't analyse empty stream
                return;
            }
            n
        }
        // There is nothing we could do, end connection.
        Err(_) => return,
    };
    let frames = Frames::from(&buffer[..nbytes]);
    for frame in frames {
        match frame {
            Frame::Message(msg) => {
                // Receiving channel was closed, so there is no point in sending this
                // frame, any of this frames and accept new connections, since whoever
                // is listening to us has stopped doing it. Send signal to self to stop running.
                if message_sender.send(msg).await.is_err() {
                    // If receiving channel is closed that means that another task
                    // has already sent termination message and it was enforced.
                    // So it doesn't matter that we failed.
                    let _ = cancelation_sender.send(()).await;
                    // Don't try to send next messages. End this task.
                    break;
                }
            }
            // We do not currently report back weather
            // parsing or execution were successful or not,
            // so for now we silently ignore any errors.
            Frame::Error => continue,
        }
    }
}
