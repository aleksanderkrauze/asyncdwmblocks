//! This module defines [UdsServer] and it's Error.

use std::error::Error;
use std::fmt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc::{self, Sender};

use super::{
    frame::{Frame, Frames},
    Server,
};
use crate::config::Config;
use crate::statusbar::BlockRefreshMessage;

/// [UdsServer]'s error. Currently it's a wrapper around [std::io::Error].
#[derive(Debug)]
pub enum UdsServerError {
    /// IO Error.
    IO(io::Error),
}

impl From<io::Error> for UdsServerError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for UdsServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg: String = match self {
            UdsServerError::IO(err) => {
                let mut msg = format!("io error: {}", err);

                if err.kind() == io::ErrorKind::AddrInUse {
                    msg.push_str("\nCheck if another program is using it, or if another instance of asyncdwmblocks is already running.");
                }

                msg
            }
        };

        write!(f, "{}", msg)
    }
}

impl Error for UdsServerError {}

#[derive(Debug, Clone)]
pub struct UdsServer {
    config: Arc<Config>,
    sender: Sender<BlockRefreshMessage>,
}

impl UdsServer {
    pub fn new(sender: mpsc::Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self {
        Self { config, sender }
    }
}

#[async_trait]
impl Server for UdsServer {
    type Error = UdsServerError;

    async fn run(&self) -> Result<(), Self::Error> {
        let listener = UnixListener::bind(&self.config.ipc.uds.addr)?;
        let (cancelation_sender, mut cancelation_receiver) = mpsc::channel::<()>(1);

        loop {
            let mut stream = tokio::select! {
                accepted_stream = listener.accept() => {
                    let (stream, _) = accepted_stream?;
                    stream
                }
                _ = cancelation_receiver.recv() => break
            };

            let cancelation_sender = cancelation_sender.clone();
            let message_sender = self.sender.clone();
            tokio::spawn(async move {
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
    use chrono::{DateTime, Utc};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tokio::io::AsyncWriteExt;
    use tokio::net::UnixStream;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn run_uds_server() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!("/tmp/asyncdwmblocks_test-{}.socket", timestamp));

        let (sender, mut receiver) = channel(8);
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let server = UdsServer::new(sender, Arc::clone(&config));
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        tokio::spawn(async move {
            let mut stream = UnixStream::connect(&config.ipc.uds.addr).await.unwrap();

            stream
                .write_all(b"REFRESH date\r\nBUTTON 3 weather\r\n")
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("date"), BlockRunMode::Normal)
        );
        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("weather"), BlockRunMode::Button(3))
        );
    }
}
