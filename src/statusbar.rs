//! This module defines [StatusBar] and it's errors.

use std::collections::HashMap;
use std::default::Default;
use std::error::Error;
use std::fmt;

use futures::future::join_all;

use crate::block::Block;

/// This struct represents an error that happened during creation of [`StatusBar`].
///
/// this struct contains informations about repeated ids, that could be treated as
/// warning or hard error. It provides a method [recover](StatusBarCreationError::recover)
/// that allows to get `StatusBar` from this error as if every `Block` had a unique id.
/// If there are multiple blocks with identical id, then only first one of them is preserved.
///
/// # Example
/// ```
/// use asyncdwmblocks::block::Block;
/// use asyncdwmblocks::statusbar::StatusBar;
///
/// # fn main() {
/// let b1 = Block::new("test".into(), "".into(), vec![], None);
/// let b2 = Block::new("test".into(), "".into(), vec![], None);
///
/// let statusbar = StatusBar::new(vec![b1, b2], " ".into());
/// assert!(statusbar.is_err());
/// let statusbar = statusbar.unwrap_err().recover();
/// # }
/// ```
#[derive(Debug)]
pub struct StatusBarCreationError {
    blocks: StatusBar,
    errors: HashMap<String, usize>,
}

/// Consumes `self` and returns `StatusBar` with `Block`s that have a unique id.
impl StatusBarCreationError {
    pub fn recover(self) -> StatusBar {
        self.blocks
    }
}

impl fmt::Display for StatusBarCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut msg =
            "Each block has to have a unique id, but some of them were identical:\n".to_string();
        for (id, num) in &self.errors {
            msg.push_str(&format!("Identifier `{}` occurs {} times\n", id, num));
        }

        write!(f, "{}", msg)
    }
}

impl Error for StatusBarCreationError {}

/// This struct represents a status bar.
///
/// `StatusBar` is a collection of `Block`s that can refresh them at
/// their interval and also listen to incoming requests to refresh
/// specific block. Each `Block` must have a unique id, witch is checked
/// at the moment of creation. It has also a delimiter, that is put
/// between each pair of adjacent blocks.
#[derive(Debug, PartialEq)]
pub struct StatusBar {
    blocks: Vec<Block>,
    delimiter: String,
}

impl StatusBar {
    /// Creates new `StatusBar` from vector of `Block`s.
    /// Returns `Ok` on success and `Err` if some blocks have unique id.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::block::Block;
    /// use asyncdwmblocks::statusbar::StatusBar;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let battery = Block::new("battery".into(), "my_battery_script".into(), vec![], Some(60));
    /// let datetime = Block::new("datetime".into(), "my_daterime_script".into(), vec![], Some(60));
    /// let info = Block::new("info".into(), "echo".into(), vec!["asyncdwmblocks".into()], None);
    ///
    /// let statusbar = StatusBar::new(vec![battery, datetime, info], " ".into())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(blocks: Vec<Block>, delimiter: String) -> Result<Self, StatusBarCreationError> {
        let mut duplicates = false;
        let mut errors: HashMap<String, usize> = HashMap::new();
        let filtered_blocks: Vec<Block> = blocks
            .into_iter()
            .filter(|b| match errors.get(b.id()).copied() {
                Some(n) => {
                    errors.insert(b.id().clone(), n + 1);
                    duplicates = true;
                    false
                }
                None => {
                    errors.insert(b.id().clone(), 1);
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
            Err(StatusBarCreationError {
                blocks: parsed_blocks,
                errors,
            })
        } else {
            Ok(parsed_blocks)
        }
    }

    /// Collects `Block`s results and concatenates them into String.
    ///
    /// If `Block`s result is `None` then this block is skipped.
    /// If non of the blocks executed it's command and empty String
    /// is returned.
    pub fn get_status_bar(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| b.result().is_some())
            .map(|b| b.result().as_ref().unwrap().clone()) // check if this clone is necessary
            // TODO: rewrite this to avoid realocarion
            .reduce(|mut acc, b| {
                acc.push_str(&self.delimiter);
                acc.push_str(&b);
                acc
            })
            .unwrap_or_default()
    }

    /// Initialises all `Block`s by awaiting completion of [running](Block::run) them.
    pub async fn init(&mut self) {
        let futures = self.blocks.iter_mut().map(|b| b.run()).collect::<Vec<_>>();

        let _ = join_all(futures).await;
    }
}

impl Default for StatusBar {
    /// Creates `StatusBar` with no blocks and a single space delimiter.
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

    fn setup_blocks_for_get_status_bar(delimiter: &str, data: Vec<Option<&str>>) -> StatusBar {
        let blocks: Vec<Block> = data
            .iter()
            .map(|x| x.map(|x| x.to_string()))
            .map(|x| {
                let mut block = Block::new("".into(), "".into(), vec![], None);
                block.set_result(x);
                block
            })
            .collect();

        StatusBar {
            blocks,
            delimiter: String::from(delimiter),
        }
    }

    #[test]
    fn statusbar_get_status_bar() {
        let statusbar =
            setup_blocks_for_get_status_bar(" ", vec![Some("A"), Some("B b B"), None, Some("D--")]);
        assert_eq!(String::from("A B b B D--"), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_empty() {
        let statusbar = StatusBar::default();
        assert_eq!(String::from(""), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_all_none() {
        let statusbar = setup_blocks_for_get_status_bar(" ", vec![None, None, None, None, None]);
        assert_eq!(String::from(""), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_emojis() {
        let statusbar = setup_blocks_for_get_status_bar(
            " | ",
            vec![Some("🔋 50%"), Some("📅 01/01/2022"), Some("🕒 12:00")],
        );
        assert_eq!(
            String::from("🔋 50% | 📅 01/01/2022 | 🕒 12:00"),
            statusbar.get_status_bar()
        );
    }

    #[test]
    fn statusbar_new_empty() {
        let statusbar = StatusBar::new(vec![], " ".into());
        assert!(statusbar.is_ok());
        assert_eq!(statusbar.unwrap().blocks, vec![]);
    }

    #[test]
    fn statusbar_new_ok() {
        let data = vec![
            Block::new("battery".into(), "".into(), vec![], None),
            Block::new("date".into(), "".into(), vec![], None),
            Block::new("time".into(), "".into(), vec![], None),
        ];
        let cloned_data = data.clone();

        let statusbar = StatusBar::new(data, " ".into());
        assert!(statusbar.is_ok());
        assert_eq!(statusbar.unwrap().blocks, cloned_data);
    }

    #[test]
    fn statusbar_new_err() {
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

        let statusbar = StatusBar::new(data, delimiter.clone());
        assert!(statusbar.is_err());
        let err = statusbar.unwrap_err();
        assert_eq!(err.errors, expected_errors);
        let recovered_statusbar = err.recover();
        assert_eq!(
            recovered_statusbar,
            StatusBar {
                blocks: unique_data,
                delimiter
            }
        );
    }

    #[tokio::test]
    async fn statusbar_init() {
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

        let mut statusbar = StatusBar::new(vec![date_block, info_block], " | ".into()).unwrap();
        statusbar.init().await;

        assert_eq!(
            statusbar.get_status_bar(),
            format!("{} | asyncdwmblocks v1", current_date)
        );
    }
}
