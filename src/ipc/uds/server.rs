//! This module defines [UdsServer] and it's Error.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc,
};

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
                    let s = concat!(
                        "\n\n",
                        "Check if another program is using it, ",
                        "or if another instance of asyncdwmblocks is already running.\n",
                        "If asyncdwmblocks is not running that means that socket file wasn't ",
                        "successfully deleted.\n",
                        "Do it and retry running asyncdwmblocks."
                    );
                    msg.push_str(s);
                }

                msg
            }
        };

        write!(f, "{}", msg)
    }
}

impl Error for UdsServerError {}

#[cfg(test)]
impl UdsServerError {
    pub(crate) fn into_io_error(self) -> Option<io::Error> {
        #[allow(unreachable_patterns)]
        match self {
            Self::IO(error) => Some(error),
            _ => None,
        }
    }
}

/// Unix domain socket [Server].
///
/// This server, once started running, will continuously do so
/// until an error will occur or termination signal was sent by
/// it's caller. It implements Drop, where it unlinks socket file
/// from the filesystem. If Drop doesn't run, then this socket file
/// will remain in the system and prevent other instances of asyncdwmblocks
/// to be run.
///
/// This server doesn't implement `Clone`, because tokio's
/// [broadcast::Receiver] doesn't implement it.
#[derive(Debug)]
pub struct UdsServer {
    config: Arc<Config>,
    sender: mpsc::Sender<BlockRefreshMessage>,
    termination_signal_receiver: broadcast::Receiver<()>,
    binded: bool,
}

impl UdsServer {
    /// Creates new Unix domain socket server.
    ///
    /// **sender** is a sender half of the channel used to
    /// communicate that some request was made.
    ///
    /// **termination_signal_receiver** is a receiver that gets
    /// notified when a OS signal was sent to this process
    /// (done by the caller).
    pub fn new(
        sender: mpsc::Sender<BlockRefreshMessage>,
        termination_signal_receiver: broadcast::Receiver<()>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            config,
            sender,
            termination_signal_receiver,
            binded: false,
        }
    }
}

#[async_trait]
impl Server for UdsServer {
    type Error = UdsServerError;

    async fn run(&mut self) -> Result<(), Self::Error> {
        let listener = UnixListener::bind(&self.config.ipc.uds.addr)?;
        self.binded = true;

        let (cancelation_sender, mut cancelation_receiver) = mpsc::channel::<()>(1);
        loop {
            let mut stream = tokio::select! {
                accepted_stream = listener.accept() => {
                    let (stream, _) = accepted_stream?;
                    stream
                }
                _ = cancelation_receiver.recv() => break,
                sig = self.termination_signal_receiver.recv() => {
                    // When we receive a termination signal we want to run
                    // cleanup code (unlinking socket file). We break from
                    // this loop and then return Ok(()) which will then in
                    // our caller run drop(server), where we perform cleanup.
                    match sig {
                        // Received signal, "terminate"
                        Ok(()) => break,
                        // If we lagged (which is very unlikely) then at least one
                        // signal was sent, "terminate"
                        Err(RecvError::Lagged(_)) => break,
                        // If channel is closed our caller does something strange.
                        // Ignore this
                        Err(RecvError::Closed) => continue,
                    }
                }
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

impl Drop for UdsServer {
    fn drop(&mut self) {
        // Unlink socket file only if we connected to it.
        // This prevens us from deleting socket file that
        // another process is using (and we falied to bind to it).
        if self.binded {
            // Ignore errors during cleanup
            let _ = fs::remove_file(&self.config.ipc.uds.addr);
        }
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
    use tokio::sync::oneshot;
    use tokio::time;

    #[tokio::test]
    async fn run_uds_server() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-server-{}.socket",
            timestamp
        ));

        let (sender, mut receiver) = mpsc::channel(8);
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let (_, termination_signal_receiver) = broadcast::channel(8);

        let mut server = UdsServer::new(sender, termination_signal_receiver, Arc::clone(&config));
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

        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("date"), BlockRunMode::Normal)
        );
        assert_eq!(
            receiver.recv().await.unwrap(),
            BlockRefreshMessage::new(String::from("weather"), BlockRunMode::Button(3))
        );
    }

    #[tokio::test]
    async fn uds_server_binding_error() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-server-binding-error-{}.socket",
            timestamp
        ));

        let (sender1, _) = mpsc::channel(8);
        let (sender2, _) = mpsc::channel(8);

        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let (termination_signal_sender, termination_signal_receiver) = broadcast::channel(8);
        let termination_signal_receiver2 = termination_signal_sender.subscribe();

        let mut server1 = UdsServer::new(sender1, termination_signal_receiver, Arc::clone(&config));
        tokio::spawn(async move {
            let _ = server1.run().await;
        });

        time::sleep(time::Duration::from_millis(100)).await;

        let mut server2 =
            UdsServer::new(sender2, termination_signal_receiver2, Arc::clone(&config));
        let s = server2.run().await;

        assert!(s.is_err());
        assert_eq!(
            s.unwrap_err().into_io_error().unwrap().kind(),
            io::ErrorKind::AddrInUse
        );
    }

    #[tokio::test]
    async fn uds_server_cleanup_on_drop() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-server-cleanup-on-drop-{}.socket",
            timestamp
        ));

        let (sender, _) = mpsc::channel(8);
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let (_, termination_signal_receiver) = broadcast::channel(8);
        let (terminate_sender, mut terminate_receiver) = oneshot::channel::<()>();

        let mut server = UdsServer::new(sender, termination_signal_receiver, Arc::clone(&config));
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = server.run() => {},
                _ = &mut terminate_receiver => {},
            }
        });

        time::sleep(time::Duration::from_millis(100)).await;
        terminate_sender.send(()).unwrap();
        handle.await.unwrap();

        assert!(!&config.ipc.uds.addr.exists());
    }

    #[tokio::test]
    async fn uds_server_cleanup_on_termination_signal() {
        let timestamp: DateTime<Utc> = DateTime::from(SystemTime::now());
        let timestamp = timestamp.format("%s").to_string();
        let addr = PathBuf::from(format!(
            "/tmp/asyncdwmblocks_test-server-cleanup-on-signal-{}.socket",
            timestamp
        ));

        let (sender, _) = mpsc::channel(8);
        let config = Config {
            ipc: config::ConfigIpc {
                server_type: ServerType::UnixDomainSocket,
                uds: config::ConfigIpcUnixDomainSocket { addr },
                ..config::ConfigIpc::default()
            },
            ..Config::default()
        }
        .arc();

        let (termination_signal_sender, termination_signal_receiver) = broadcast::channel(8);

        let mut server = UdsServer::new(sender, termination_signal_receiver, Arc::clone(&config));
        let handle = tokio::spawn(async move {
            server.run().await.unwrap();
        });

        time::sleep(time::Duration::from_millis(100)).await;
        termination_signal_sender.send(()).unwrap();
        handle.await.unwrap();

        assert!(!&config.ipc.uds.addr.exists());
    }
}
