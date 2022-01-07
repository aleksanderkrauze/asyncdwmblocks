use std::error::Error;
use std::fmt;
use std::net::Ipv4Addr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::{self, Sender};

use crate::config::Config;
use crate::server::{
    frame::{Frame, Frames},
    Listener,
};
use crate::statusbar::BlockRefreshMessage;

#[derive(Debug)]
pub enum TcpListenerError {
    IO(std::io::Error),
}

impl From<std::io::Error> for TcpListenerError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl fmt::Display for TcpListenerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg: String = match self {
            TcpListenerError::IO(err) => format!("io error: {}", err),
        };

        write!(f, "{}", msg)
    }
}

impl Error for TcpListenerError {}

#[derive(Debug, Clone)]
pub struct TcpListener {
    config: Arc<Config>,
    sender: Sender<BlockRefreshMessage>,
}

#[async_trait]
impl Listener for TcpListener {
    type Error = TcpListenerError;
    fn new(sender: Sender<BlockRefreshMessage>, config: Arc<Config>) -> Self {
        Self { sender, config }
    }

    async fn run(&self) -> Result<(), Self::Error> {
        let listener =
            tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, self.config.tcp_port)).await?;
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
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn run_tcp_listener() {
        let (sender, mut receiver) = channel(8);
        let config = Arc::new(Config {
            tcp_port: 44002,
            ..Config::default()
        });

        let listener = TcpListener::new(sender, Arc::clone(&config));
        tokio::spawn(async move {
            let _ = listener.run().await;
        });

        tokio::spawn(async move {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, config.tcp_port))
                .await
                .unwrap();

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
}
