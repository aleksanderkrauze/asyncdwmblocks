use std::collections::HashMap;
use std::default::Default;
use std::error::Error;
use std::fmt;

use tokio::process::Command;
use tokio::time::{interval, Duration, Interval, MissedTickBehavior};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    identifier: String,
    command: String,
    args: Vec<String>,
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

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let output = Command::new(&self.command)
            .args(&self.args)
            .output()
            .await?
            .stdout;
        let result = String::from_utf8_lossy(&output).to_string();
        self.result = Some(result);

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
            .filter(|b| {
                // Mapping is required to satisfy "mutable_borrow_reservation_conflict" lint.
                // See: https://github.com/rust-lang/rust/issues/59159
                match errors.get(&b.identifier).map(|n| *n) {
                    Some(n) => {
                        errors.insert(b.identifier.clone(), n + 1);
                        duplicates = true;
                        false
                    }
                    None => {
                        errors.insert(b.identifier.clone(), 1);
                        true
                    }
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
    async fn block_run() {
        let mut echo = Block::new(
            "echo-test".to_string(),
            "echo".to_string(),
            vec!["ECHO".to_string()],
            42,
        );
        assert_eq!(echo.result, None);
        echo.run().await.expect("Failed to run command.");
        assert_eq!(echo.result, Some("ECHO\n".to_string()));
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
}
