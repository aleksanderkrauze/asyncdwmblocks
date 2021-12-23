use std::collections::HashMap;
use std::default::Default;
use std::error::Error;
use std::fmt;

use futures::future::join_all;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::{interval, Duration, Interval, MissedTickBehavior};

#[derive(Debug)]
pub enum BlockRunError {
    CommandError(std::io::Error),
    JoinError(task::JoinError),
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
    pub fn is_internal(&self) -> bool {
        match self {
            BlockRunError::JoinError(_) | BlockRunError::ChannelClosed => true,
            BlockRunError::CommandError(_) => false,
        }
    }

    pub fn is_io(&self) -> bool {
        match self {
            BlockRunError::JoinError(_) | BlockRunError::ChannelClosed => false,
            BlockRunError::CommandError(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    identifier: String,
    command: String,
    args: Vec<String>,
    // TODO: make interval optional to prevent block from refreshing.
    interval: Duration,
    result: Option<String>,
}

impl Block {
    pub fn new(identifier: String, command: String, args: Vec<String>, interval: u32) -> Self {
        assert!(interval > 0, "Interval must be at least 1 second.");
        Self {
            identifier,
            command,
            args,
            interval: Duration::from_secs(interval as u64),
            result: None,
        }
    }

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
                .filter(|c| c != &'\n')
                .collect(),
        );
        Ok(())
    }

    pub fn get_scheduler(&self) -> Interval {
        let mut scheduler = interval(self.interval);
        scheduler.set_missed_tick_behavior(MissedTickBehavior::Delay);

        scheduler
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
                interval: Duration::new(0, 0),
                result: x,
            })
            .collect();

        Blocks {
            blocks,
            delimiter: String::from(delimiter),
        }
    }

    #[tokio::test]
    async fn bloks_run_error_types() {
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
            42,
        );
        assert_eq!(echo.result, None);
        echo.run().await.expect("Failed to run command.");
        assert_eq!(echo.result, Some("ECHO".to_string()));
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
            Block::new("battery".into(), "".into(), vec![], 1),
            Block::new("date".into(), "".into(), vec![], 1),
            Block::new("time".into(), "".into(), vec![], 1),
        ];
        let cloned_data = data.clone();

        let blocks = Blocks::new(data, " ".into());
        assert!(blocks.is_ok());
        assert_eq!(blocks.unwrap().blocks, cloned_data);
    }

    #[test]
    fn blocks_new_err() {
        let data = vec![
            Block::new("battery".into(), "".into(), vec![], 1),
            Block::new("date".into(), "".into(), vec![], 1),
            Block::new("date".into(), "".into(), vec![], 1),
            Block::new("time".into(), "".into(), vec![], 1),
            Block::new("time".into(), "".into(), vec![], 1),
            Block::new("time".into(), "".into(), vec![], 1),
        ];
        let unique_data = vec![
            Block::new("battery".into(), "".into(), vec![], 1),
            Block::new("date".into(), "".into(), vec![], 1),
            Block::new("time".into(), "".into(), vec![], 1),
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
        let date_block = Block::new("date".into(), "date".into(), vec!["+%d/%m/%Y".into()], 1);
        let info_block = Block::new(
            "info".into(),
            "echo".into(),
            vec!["asyncdwmblocks v1".into()],
            1,
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
