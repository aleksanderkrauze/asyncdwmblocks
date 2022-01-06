//! This module allows for chaining channels by "piping" them.
use tokio::sync::mpsc;

/// Connects two channels and translates messages between them.
///
/// This function connects first channel's receiver with second
/// channel's sender. They can have different types. Translation
/// between types is performed by `translate` closure (or function).
/// This function operates in a loop, so it has to be put on
/// separate task (see example).
///
/// # Warning
/// This pipe does not guarantee that all messages send will be received.
///
/// Because there is no way to get a notification when a receiving end of
/// a channel is closed and sends could return immediately if there is
/// place in the internal buffer, this pipe cannot guarantee that all
/// sent messages will be received.
///
/// # Example
/// ```
/// use asyncdwmblocks::pipe::mpsc_pipe_translate;
/// use tokio::sync::mpsc::channel;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let (s1, r1) = channel(4);
/// let (s2, mut r2) = channel(4);
///
/// tokio::spawn(async move {
///     mpsc_pipe_translate(r1, s2, |x| if x % 2 == 0 { "even" } else { "odd" }).await;
/// });
///
/// s1.send(42).await?;
/// s1.send(21).await?;
/// s1.send(17).await?;
///
/// assert_eq!(r2.recv().await, Some("even"));
/// assert_eq!(r2.recv().await, Some("odd"));
/// assert_eq!(r2.recv().await, Some("odd"));
/// # Ok(())
/// # }
/// ```
pub async fn mpsc_pipe_translate<R, S, T>(
    mut receiver: mpsc::Receiver<R>,
    sender: mpsc::Sender<S>,
    translate: T,
) where
    T: Fn(R) -> S,
{
    while let Some(msg) = receiver.recv().await {
        let msg = translate(msg);

        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

/// Connects two channels
///
/// This function operates in the same way that [mpsc_pipe_translate]
/// operates, but does not translate messages. For example see documentation
/// of mentioned before function.
///
/// # Warning
/// This pipe does not guarantee that all messages send will be received.
/// For explanation consult corresponding section in [mpsc_pipe_translate] documentation.
pub async fn mpsc_pipe<P>(receiver: mpsc::Receiver<P>, sender: mpsc::Sender<P>) {
    mpsc_pipe_translate(receiver, sender, |x| x).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, timeout_at, Duration, Instant};

    #[tokio::test]
    async fn simple_message_passthrough() {
        let (s1, r1) = mpsc::channel(2);
        let (s2, mut r2) = mpsc::channel(2);

        tokio::spawn(async move {
            mpsc_pipe(r1, s2).await;
        });

        s1.send(42).await.unwrap();
        assert_eq!(r2.recv().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn many_messages() {
        let (s1, r1) = mpsc::channel(2);
        let (s2, mut r2) = mpsc::channel(2);

        tokio::spawn(async move {
            mpsc_pipe(r1, s2).await;
        });

        s1.send(1).await.unwrap();
        s1.send(2).await.unwrap();
        s1.send(3).await.unwrap();
        s1.send(4).await.unwrap();
        s1.send(5).await.unwrap();
        drop(s1);
        assert_eq!(r2.recv().await.unwrap(), 1);
        assert_eq!(r2.recv().await.unwrap(), 2);
        assert_eq!(r2.recv().await.unwrap(), 3);
        assert_eq!(r2.recv().await.unwrap(), 4);
        assert_eq!(r2.recv().await.unwrap(), 5);
        assert!(r2.recv().await.is_none());
    }

    #[tokio::test]
    async fn translate_message() {
        let (s1, r1) = mpsc::channel(2);
        let (s2, mut r2) = mpsc::channel(2);

        tokio::spawn(async move {
            mpsc_pipe_translate(r1, s2, |x| 2 * x).await;
        });

        s1.send(42).await.unwrap();
        assert_eq!(r2.recv().await.unwrap(), 84);
    }

    #[tokio::test]
    async fn dropped_sender() {
        let (_, r1) = mpsc::channel::<i32>(2);
        let (s2, mut r2) = mpsc::channel::<i32>(2);

        tokio::spawn(async move {
            mpsc_pipe_translate(r1, s2, |x| 2 * x).await;
        });

        assert!(r2.recv().await.is_none());
    }

    #[tokio::test]
    async fn dropped_receiver() {
        let (s1, r1) = mpsc::channel::<i32>(2);
        let (s2, _) = mpsc::channel::<i32>(2);

        tokio::spawn(async move {
            mpsc_pipe_translate(r1, s2, |x| 2 * x).await;
        });

        assert!(s1.send(42).await.is_ok());
        sleep(Duration::from_millis(100)).await;
        assert!(s1.send(42).await.is_err());
        assert!(s1.send(42).await.is_err());
        assert!(s1.send(42).await.is_err());
    }

    #[tokio::test]
    async fn no_messages() {
        #[allow(unused_variables)]
        let (s1, r1) = mpsc::channel::<i32>(2);
        let (s2, mut r2) = mpsc::channel::<i32>(2);

        tokio::spawn(async move {
            mpsc_pipe_translate(r1, s2, |x| 2 * x).await;
        });

        let timeout = timeout_at(Instant::now() + Duration::from_millis(100), r2.recv()).await;
        assert!(timeout.is_err());
    }

    #[tokio::test]
    async fn chain() {
        let (s1, r1) = mpsc::channel::<&str>(10);
        let (s2, r2) = mpsc::channel::<(&str, usize)>(10);
        let (s3, mut r3) = mpsc::channel::<String>(10);

        // If given word has even number of characters return characters
        // at even positions (counting form 0) and if it has odd number
        // of characters return characters at odd positions.
        tokio::spawn(async move {
            mpsc_pipe_translate(r1, s2, |w| (w, w.chars().count())).await;
        });
        tokio::spawn(async move {
            mpsc_pipe_translate(r2, s3, |(word, count)| {
                word.chars()
                    .enumerate()
                    .filter(|(i, _)| i % 2 == count % 2)
                    .map(|(_, c)| c)
                    .collect()
            })
            .await;
        });

        s1.send("Bird").await.unwrap();
        s1.send("is").await.unwrap();
        s1.send("the").await.unwrap();
        s1.send("word").await.unwrap();

        assert_eq!(r3.recv().await.unwrap().as_str(), "Br");
        assert_eq!(r3.recv().await.unwrap().as_str(), "i");
        assert_eq!(r3.recv().await.unwrap().as_str(), "h");
        assert_eq!(r3.recv().await.unwrap().as_str(), "wr");
    }
}
