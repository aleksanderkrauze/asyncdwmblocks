//! This module defines [Block] type and it's errors.

use std::error::Error;
use std::fmt;

use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::{interval_at, Duration, Instant, Interval, MissedTickBehavior};

/// Error that may occur when running (and awaiting) [Block::run].
///
/// While awaiting for `Block::run()` three things could happen wrong:
///
///  1. Execution of provided command could fail (represented by `CommandError` variant).
///  2. Task spawned by `tokio` failed to finish (represented by `JoinError` variant).
///  3. Channel used to communicate stdout of running command closed before
///  sending value (represented by `ChannelClosed` variant).
///
/// Depending on witch variant happened different action might be appropriate.
/// If it is the first case then this error is probably user fault. We can then
/// choose to end program, log it, inform user or simply ignore it. If it is on
/// the other hand the latter case, then it is probably internal bug that should
/// be reported.
///
/// To help identify these cases and allow to skip pattern matching, two helping
/// methods are provided: [is_internal](BlockRunError::is_internal) and [is_io](BlockRunError::is_io).
/// They are exhaustive and mutually exclusive.
///
/// # Example
/// ```
/// use asyncdwmblocks::block::{Block, BlockRunMode};
///
/// # async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut b = Block::new("battery".into(), "my_battery_script.sh".into(), vec![], Some(60));
/// match b.run(BlockRunMode::Normal).await {
///     Ok(_) => {
///         // everything is ok.
///     }
///     Err(e) => {
///         if e.is_io() {
///             // log error and continue work.
///         } else {
///             panic!("Encountered unexpected internal error: {}", e);
///         }
///     }
/// };
///
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub enum BlockRunError {
    /// io error that happened when Command was executed.
    CommandError(std::io::Error),
    /// tokio's JoinError that happened in spawned job.
    JoinError(task::JoinError),
    /// tokio's oneshot channel was closed before it could receive computation result.
    ChannelClosed,
}

impl fmt::Display for BlockRunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            BlockRunError::CommandError(e) => e.to_string(),
            BlockRunError::JoinError(e) => e.to_string(),
            BlockRunError::ChannelClosed => "Channel was closed".to_string(),
        };

        write!(f, "{}", msg)
    }
}

impl Error for BlockRunError {}

impl From<std::io::Error> for BlockRunError {
    fn from(err: std::io::Error) -> Self {
        Self::CommandError(err)
    }
}

impl From<task::JoinError> for BlockRunError {
    fn from(err: task::JoinError) -> Self {
        Self::JoinError(err)
    }
}

impl From<oneshot::error::RecvError> for BlockRunError {
    fn from(_err: oneshot::error::RecvError) -> Self {
        Self::ChannelClosed
    }
}

impl BlockRunError {
    /// Returns true if error is internal.
    ///
    /// This means that this error should be treated as a bug
    /// as this means that either tokio or this program failed.
    pub fn is_internal(&self) -> bool {
        match self {
            BlockRunError::JoinError(_) | BlockRunError::ChannelClosed => true,
            BlockRunError::CommandError(_) => false,
        }
    }

    /// Returns true if error is external (failure to run a command).
    ///
    /// This error is probably user fault and can be ignored (if user wishes so).
    /// It could be caused by user providing wrong command, not having proper
    /// permissions to run a script, `$PATH` being wrongly set, etc.
    pub fn is_io(&self) -> bool {
        match self {
            BlockRunError::JoinError(_) | BlockRunError::ChannelClosed => false,
            BlockRunError::CommandError(_) => true,
        }
    }
}

/// This enum represents how block should be run
/// (should env var `$BUTTON` be set).
///
/// `asyncdwmblocks` gives you an ability to make your blocks
/// behave differently when clicked. This is done by setting
/// environment variable `$BUTTON` for spawned process by running
/// a block. You can therefore use this variable in your scripts
/// and choose different action when clicked with specific mouse button.
///
/// # Example
/// ```
/// use asyncdwmblocks::block::{Block, BlockRunMode};
///
/// # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut block = Block::new("date_block".into(), "date_script".into(), vec![], Some(60));
///
/// block.run(BlockRunMode::Normal).await?; // run date_script normally
/// block.run(BlockRunMode::Button(1)).await?; // run date_script and set $BUTTON to 1 (left click)
/// # Ok(())
/// # }
/// ```
#[derive(Debug, PartialEq, Clone)]
pub enum BlockRunMode {
    /// Run [`Block`] normally
    Normal,
    /// Run [`Block`] and set `$BUTTON` to inner value
    Button(u8),
}

impl BlockRunMode {
    /// Gets `$BUTTON` number or `None` if self is [BlockRunMode::Normal].
    pub fn button(&self) -> Option<u8> {
        match self {
            BlockRunMode::Button(b) => Some(*b),
            BlockRunMode::Normal => None,
        }
    }
}

// TODO: If result is &self and run is &mut self does it mean that
// we can't get past result while we are await current computation?

/// This struct represents single status bar block.
#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    id: String,
    command: String,
    args: Vec<String>,
    interval: Option<Duration>,
    result: Option<String>,
}

impl Block {
    /// Creates a new `Block`.
    ///
    /// Required arguments have following meaning:
    ///  - `id`: id of this block
    ///  - `command`: command that should be executed every time this block is reloaded
    ///  - `args`: arguments to this command
    ///  - `interval`: at witch rate (in seconds) this block should reload.
    ///  If `None` then it won't be automatically reload (but still can be by sending
    ///  proper signal to status bar)
    ///
    ///  # Panics
    ///  If `interval` is `Some`, then it must be greater than 0. Interval with value
    ///  `Some(0)` will panic.
    pub fn new(id: String, command: String, args: Vec<String>, interval: Option<u64>) -> Self {
        // TODO: make new accept Cows instead of Strings.
        if interval.is_some() {
            assert!(interval > Some(0), "Interval must be at least 1 second.");
        }
        Self {
            id,
            command,
            args,
            interval: interval.map(Duration::from_secs),
            result: None,
        }
    }

    /// Executes Block's command by running tokio's **`spawn_blocking`**.
    ///
    /// This method runs Block's command (with it's args) and returns `Ok(())`
    /// on success and `Err(BlockRunError)` on failure. Consult [it's](BlockRunError)
    /// documentation for more details. [mode](BlockRunMode) indicates if environment
    /// variable `$BUTTON` should be set.
    ///
    /// If succeeded it takes characters from command's output (stdout) up to first
    /// newline character and then sets it as a inner result.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::block::{Block, BlockRunMode};
    ///
    /// # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut block = Block::new("hello".into(), "echo".into(), vec!["Hello".into()], None);
    /// block.run(BlockRunMode::Normal).await?;
    ///
    /// assert_eq!(block.result(), Some(&String::from("Hello")));
    /// # Ok(())
    /// # }
    ///
    /// ```
    pub async fn run(&mut self, mode: BlockRunMode) -> Result<(), BlockRunError> {
        let (sender, receiver) = oneshot::channel();

        let command = self.command.clone();
        let args = self.args.clone();

        task::spawn_blocking(|| async move {
            let mut command = Command::new(command);
            let command = command.args(args);
            let command = match mode.button() {
                Some(b) => command.env("BUTTON", b.to_string()),
                None => command,
            };

            // ignore sending error
            let _ = sender.send(command.output().await.map(|o| o.stdout));
        })
        .await?
        .await;

        let output: Vec<u8> = receiver.await??;

        self.result = Some(
            String::from_utf8_lossy(&output)
                .chars()
                .take_while(|c| c != &'\n')
                .collect(),
        );
        Ok(())
    }

    /// Creates properly configured [Interval] that ticks at Block's rate.
    ///
    /// If upon creation `interval` was set to `None` (meaning no refreshment)
    /// this method will return `None` as well.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::block::Block;
    ///
    /// # use std::time::Duration;
    /// # async fn async_main() {
    /// let date = Block::new("date".into(), "date".into(), vec![], Some(60));
    /// let message = Block::new("hello_message".into(), "echo".into(), vec!["Hello!".into()], None);
    ///
    /// assert_eq!(date.get_scheduler().unwrap().period(), Duration::from_secs(60));
    /// assert!(message.get_scheduler().is_none());
    /// # }
    /// ```
    pub fn get_scheduler(&self) -> Option<Interval> {
        let interval = self.interval?;
        let mut scheduler = interval_at(Instant::now() + interval, interval);
        scheduler.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Some(scheduler)
    }

    /// Returns reference to a result of a previous computation.
    /// `None` means that no computation has ever been completed.
    pub fn result(&self) -> Option<&String> {
        self.result.as_ref()
    }

    /// Returns reference to Block's id.
    pub fn id(&self) -> &String {
        &self.id
    }
}

#[cfg(test)]
impl Block {
    pub(crate) fn set_result(&mut self, result: Option<String>) {
        self.result = result;
    }

    pub(crate) fn get_interval(&self) -> Option<Duration> {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout_at;

    #[tokio::test]
    async fn block_run_error_types() {
        use BlockRunError::*;

        let command_error = CommandError(std::io::Error::new(std::io::ErrorKind::Other, "testing"));
        let channel_closed = ChannelClosed;
        // This is the only way I know to create a JoinError
        let join_error = tokio::spawn(async { panic!() }).await.unwrap_err();
        let join_error = JoinError(join_error);

        assert_eq!(command_error.is_io(), true);
        assert_eq!(command_error.is_internal(), false);

        assert_eq!(channel_closed.is_io(), false);
        assert_eq!(channel_closed.is_internal(), true);

        assert_eq!(join_error.is_io(), false);
        assert_eq!(join_error.is_internal(), true);
    }

    #[tokio::test]
    async fn block_run() {
        let mut echo = Block::new(
            "echo-test".to_string(),
            "echo".to_string(),
            vec!["ECHO".to_string()],
            None,
        );
        assert_eq!(echo.result, None);
        echo.run(BlockRunMode::Normal)
            .await
            .expect("Failed to run command.");
        assert_eq!(echo.result, Some("ECHO".to_string()));
    }

    #[tokio::test]
    async fn block_run_multiple_lines() {
        let mut echo = Block::new(
            "echo-test".to_string(),
            "echo".to_string(),
            vec!["LINE1\nLINE2".to_string()],
            None,
        );
        assert_eq!(echo.result, None);
        echo.run(BlockRunMode::Normal)
            .await
            .expect("Failed to run command.");
        assert_eq!(echo.result, Some("LINE1".to_string()));
    }

    #[tokio::test]
    async fn run_nonexisting_command() {
        let mut block = Block::new(
            "error".into(),
            "xfewxj1287rxn31xm31rx798321x".into(),
            vec![],
            None,
        );
        let run = block.run(BlockRunMode::Normal).await;
        assert!(run.is_err());
        assert!(run.unwrap_err().is_io());
    }

    #[tokio::test]
    async fn run_test_blocking() {
        let mut block = Block::new("".into(), "sleep".into(), vec!["1".into()], None);

        let timeout = timeout_at(
            Instant::now() + Duration::from_millis(10),
            block.run(BlockRunMode::Normal),
        )
        .await;
        assert!(timeout.is_err());

        let timeout = timeout_at(
            Instant::now() + Duration::from_secs(1) + Duration::from_millis(10),
            block.run(BlockRunMode::Normal),
        )
        .await;
        assert!(timeout.is_ok());
    }

    #[tokio::test]
    async fn block_get_scheduler() {
        let block = Block::new("".into(), "".into(), vec![], Some(1));
        let mut scheduler = block.get_scheduler().unwrap();

        let timeout =
            timeout_at(Instant::now() + Duration::from_millis(10), scheduler.tick()).await;

        assert!(timeout.is_err());

        let timeout = timeout_at(Instant::now() + Duration::from_secs(1), scheduler.tick()).await;

        assert!(timeout.is_ok());
    }
}
