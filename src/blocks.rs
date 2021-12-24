//! This module defines [Block] and [Blocks] types and their errors.

use std::collections::HashMap;
use std::default::Default;
use std::error::Error;
use std::fmt;

use futures::future::join_all;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::{interval, Duration, Interval, MissedTickBehavior};

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
/// use asyncdwmblocks::blocks::Block;
/// # async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let mut b = Block::new("battery".into(), "my_battery_script.sh".into(), vec![], Some(60));
/// match b.run().await {
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

/// This struct represents single status bar block.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    identifier: String,
    command: String,
    args: Vec<String>,
    interval: Option<Duration>,
    result: Option<String>,
}

impl Block {
    /// Creates a new `Block`.
    ///
    /// Required arguments have following meaning:
    ///  - `identifier`: id of this block
    ///  - `command`: command that should be executed every time this block is reloaded
    ///  - `args`: arguments to this command
    ///  - `interval`: at witch rate (in seconds) this block should reload.
    ///  If `None` then it won't be automatically reload (but still can be by sending
    ///  proper signal to status bar)
    ///
    ///  # Panics
    ///  If `interval` is `Some`, then it must be greater than 0. Interval with value
    ///  `Some(0)` will panic.
    pub fn new(
        identifier: String,
        command: String,
        args: Vec<String>,
        interval: Option<u64>,
    ) -> Self {
        // TODO: make new accept Cows instead of Strings.
        if interval.is_some() {
            assert!(interval > Some(0), "Interval must be at least 1 second.");
        }
        Self {
            identifier,
            command,
            args,
            interval: interval.map(|i| Duration::from_secs(i)),
            result: None,
        }
    }

    /// Executes Block's command by running tokio's **`spawn_blocking`**.
    ///
    /// This method runs Block's command (with it's args) and returns `Ok(())`
    /// on success and `Err(BlockRunError)` on failure. Consult [it's](BlockRunError)
    /// documentation for more details.
    ///
    /// If succeeded it takes characters from command's output (stdout) up to first
    /// newline character and then sets it as a inner result.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::blocks::Block;
    ///
    /// # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut block = Block::new("hello".into(), "echo".into(), vec!["Hello".into()], None);
    /// block.run().await?;
    ///
    /// assert_eq!(block.result(), &Some(String::from("Hello")));
    /// # Ok(())
    /// # }
    ///
    /// ```
    pub async fn run(&mut self) -> Result<(), BlockRunError> {
        let (sender, receiver) = oneshot::channel();

        let command = self.command.clone();
        let args = self.args.clone();

        task::spawn_blocking(move || async {
            // ignore sending error
            let _ = sender.send(
                Command::new(command)
                    .args(args)
                    .output()
                    .await
                    .map(|o| o.stdout),
            );
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
    /// use asyncdwmblocks::blocks::Block;
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
        let mut scheduler = interval(self.interval?);
        scheduler.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Some(scheduler)
    }

    /// Returns reference to a result of a previous computation.
    /// `None` means that no computation has ever been completed.
    pub fn result(&self) -> &Option<String> {
        &self.result
    }
}

#[derive(Debug)]
pub struct BlocksCreationError {
    blocks: Blocks,
    errors: HashMap<String, usize>,
}

impl BlocksCreationError {
    pub fn recover(self) -> Blocks {
        self.blocks
    }
}

impl fmt::Display for BlocksCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut msg =
            "Each block has to have a unique identifier, but some of them were identical:\n"
                .to_string();
        for (id, num) in &self.errors {
            msg.push_str(&format!("Identifier `{}` occurs {} times\n", id, num));
        }

        write!(f, "{}", msg)
    }
}

impl Error for BlocksCreationError {}

#[derive(Debug, PartialEq)]
pub struct Blocks {
    blocks: Vec<Block>,
    delimiter: String,
}

impl Blocks {
    pub fn new(blocks: Vec<Block>, delimiter: String) -> Result<Self, BlocksCreationError> {
        let mut duplicates = false;
        let mut errors: HashMap<String, usize> = HashMap::new();
        let filtered_blocks: Vec<Block> = blocks
            .into_iter()
            .filter(|b| match errors.get(&b.identifier).copied() {
                Some(n) => {
                    errors.insert(b.identifier.clone(), n + 1);
                    duplicates = true;
                    false
                }
                None => {
                    errors.insert(b.identifier.clone(), 1);
                    true
                }
            })
            .collect();

        let parsed_blocks = Self {
            blocks: filtered_blocks,
            delimiter,
        };
        if duplicates {
            let errors: HashMap<String, usize> =
                errors.into_iter().filter(|(_, n)| n > &1).collect();
            Err(BlocksCreationError {
                blocks: parsed_blocks,
                errors,
            })
        } else {
            Ok(parsed_blocks)
        }
    }

    pub fn get_status_bar(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| b.result.is_some())
            .map(|b| b.result.as_ref().unwrap().clone()) // check if this clone is necessary
            // TODO: rewrite this to avoid realocarion
            .reduce(|mut acc, b| {
                acc.push_str(&self.delimiter);
                acc.push_str(&b);
                acc
            })
            .unwrap_or_default()
    }

    pub async fn init(&mut self) {
        let futures = self.blocks.iter_mut().map(|b| b.run()).collect::<Vec<_>>();

        let _ = join_all(futures).await;
    }
}

impl Default for Blocks {
    fn default() -> Self {
        Self {
            blocks: Vec::default(),
            delimiter: String::from(" "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use std::time::SystemTime;

    fn setup_blocks_for_get_status_bar(delimiter: &str, data: Vec<Option<&str>>) -> Blocks {
        let blocks: Vec<Block> = data
            .iter()
            .map(|x| x.map(|x| x.to_string()))
            .map(|x| Block {
                identifier: String::new(),
                command: String::new(),
                args: Vec::new(),
                interval: None,
                result: x,
            })
            .collect();

        Blocks {
            blocks,
            delimiter: String::from(delimiter),
        }
    }

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
        echo.run().await.expect("Failed to run command.");
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
        echo.run().await.expect("Failed to run command.");
        assert_eq!(echo.result, Some("LINE1".to_string()));
    }

    #[test]
    fn blocks_get_status_bar() {
        let blocks =
            setup_blocks_for_get_status_bar(" ", vec![Some("A"), Some("B b B"), None, Some("D--")]);
        assert_eq!(String::from("A B b B D--"), blocks.get_status_bar());
    }

    #[test]
    fn blocks_get_status_bar_empty() {
        let blocks = Blocks::default();
        assert_eq!(String::from(""), blocks.get_status_bar());
    }

    #[test]
    fn blocks_get_status_bar_all_none() {
        let blocks = setup_blocks_for_get_status_bar(" ", vec![None, None, None, None, None]);
        assert_eq!(String::from(""), blocks.get_status_bar());
    }

    #[test]
    fn blocks_get_status_bar_emojis() {
        let blocks = setup_blocks_for_get_status_bar(
            " | ",
            vec![Some("🔋 50%"), Some("📅 01/01/2022"), Some("🕒 12:00")],
        );
        assert_eq!(
            String::from("🔋 50% | 📅 01/01/2022 | 🕒 12:00"),
            blocks.get_status_bar()
        );
    }

    #[test]
    fn blocks_new_empty() {
        let blocks = Blocks::new(vec![], " ".into());
        assert!(blocks.is_ok());
        assert_eq!(blocks.unwrap().blocks, vec![]);
    }

    #[test]
    fn blocks_new_ok() {
        let data = vec![
            Block::new("battery".into(), "".into(), vec![], None),
            Block::new("date".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
        ];
        let cloned_data = data.clone();

        let blocks = Blocks::new(data, " ".into());
        assert!(blocks.is_ok());
        assert_eq!(blocks.unwrap().blocks, cloned_data);
    }

    #[test]
    fn blocks_new_err() {
        let data = vec![
            Block::new("battery".into(), "".into(), vec![], None),
            Block::new("date".into(), "".into(), vec![], None),
            Block::new("date".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
        ];
        let unique_data = vec![
            Block::new("battery".into(), "".into(), vec![], None),
            Block::new("date".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
        ];
        let expected_errors: HashMap<String, usize> = vec![("date".into(), 2), ("time".into(), 3)]
            .into_iter()
            .collect();
        let delimiter = String::from(" ");

        let blocks = Blocks::new(data, delimiter.clone());
        assert!(blocks.is_err());
        let err = blocks.unwrap_err();
        assert_eq!(err.errors, expected_errors);
        let recovered_blocks = err.recover();
        assert_eq!(
            recovered_blocks,
            Blocks {
                blocks: unique_data,
                delimiter
            }
        );
    }

    #[tokio::test]
    async fn blocks_init() {
        // Flag -u sets UTC standard. Since this is what we are comparing
        // this must be set, or this test will fail around midnight.
        let date_block = Block::new(
            "date".into(),
            "date".into(),
            vec!["-u".into(), "+%d/%m/%Y".into()],
            None,
        );
        let info_block = Block::new(
            "info".into(),
            "echo".into(),
            vec!["asyncdwmblocks v1".into()],
            None,
        );

        let current_date: DateTime<Utc> = DateTime::from(SystemTime::now());
        let current_date = current_date.format("%d/%m/%Y").to_string();

        let mut blocks = Blocks::new(vec![date_block, info_block], " | ".into()).unwrap();
        blocks.init().await;

        assert_eq!(
            blocks.get_status_bar(),
            format!("{} | asyncdwmblocks v1", current_date)
        );
    }
}
