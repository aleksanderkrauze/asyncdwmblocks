//! This module defines [StatusBar] and it's errors.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use futures::future::join_all;
use tokio::sync::mpsc;

use crate::block::{Block, BlockRunMode};

/// This struct represents an error that happened during creation of [`StatusBar`].
///
/// This struct contains informations about repeated ids, that could be treated as
/// warning or hard error. It provides a method [recover](StatusBarCreationError::recover)
/// that allows to get `StatusBar` from this error as if every `Block` had a unique id.
/// If there are multiple blocks with identical id, then only first one of them is preserved.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use asyncdwmblocks::block::Block;
/// use asyncdwmblocks::statusbar::StatusBar;
/// use asyncdwmblocks::config::Config;
///
/// # fn main() {
/// let config = Arc::new(Config::default());
/// let b1 = Block::new("test".into(), "".into(), vec![], None, Arc::clone(&config));
/// let b2 = Block::new("test".into(), "".into(), vec![], None, Arc::clone(&config));
///
/// let statusbar = StatusBar::new(vec![b1, b2], " ".into());
/// assert!(statusbar.is_err());
/// let statusbar = statusbar.unwrap_err().recover();
/// # }
/// ```
#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct BlockRefreshMessage {
    name: String,
    mode: BlockRunMode,
}

impl BlockRefreshMessage {
    pub fn new(name: String, mode: BlockRunMode) -> Self {
        Self { name, mode }
    }

    pub(self) fn name(&self) -> &String {
        &self.name
    }

    pub(self) fn mode(&self) -> &BlockRunMode {
        &self.mode
    }
}

/// This struct represents a status bar.
///
/// `StatusBar` is a collection of `Block`s that can refresh them at
/// their interval and also listen to incoming requests to refresh
/// specific block. Each `Block` must have a unique id, witch is checked
/// at the moment of creation. It has also a delimiter, that is put
/// between each pair of adjacent blocks.
#[derive(Debug, PartialEq, Clone)]
pub struct StatusBar {
    blocks: Vec<Block>,
    delimiter: String,
    buff_size: Option<usize>,
}

impl StatusBar {
    /// Creates new `StatusBar` from vector of `Block`s.
    /// Returns `Ok` on success and `Err` if some blocks have unique id.
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    /// use asyncdwmblocks::block::Block;
    /// use asyncdwmblocks::statusbar::StatusBar;
    /// use asyncdwmblocks::config::Config;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Arc::new(Config::default());
    /// let battery = Block::new("battery".into(), "my_battery_script".into(), vec![], Some(60), Arc::clone(&config));
    /// let datetime = Block::new("datetime".into(), "my_daterime_script".into(), vec![], Some(60), Arc::clone(&config));
    /// let info = Block::new("info".into(), "echo".into(), vec!["asyncdwmblocks".into()], None, Arc::clone(&config));
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
            buff_size: None,
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

    /// Starts executing blocks asynchronously and sending results through a channel.
    ///
    /// This function requires two channel pairs to be created. One to send results of
    /// a status bar computation (**sender**) and the other to signal reloading specific
    /// block (**reload**). This function can possibly run to infinity
    /// (if there is at least one block with `Some` interval) and so it should be either
    /// spawned as a separate task, or should be placed at the end of method call.
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    /// use tokio::sync::mpsc;
    /// use asyncdwmblocks::block::Block;
    /// use asyncdwmblocks::statusbar::StatusBar;
    /// use asyncdwmblocks::config::Config;
    ///
    /// # async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Arc::new(Config::default());
    /// let b = Block::new("date_block".into(), "date".into(), vec![], Some(60), Arc::clone(&config));
    /// let mut status_bar = StatusBar::new(vec![b], " ".into())?;
    ///
    /// let (result_sender, mut result_receiver) = mpsc::channel(8);
    /// let (reload_sender, reload_receiver) = mpsc::channel(8);
    ///
    /// tokio::spawn(async move {
    ///     status_bar.run(result_sender, reload_receiver).await;
    /// });
    ///
    /// while let Some(_) = result_receiver.recv().await {
    ///  // do stuff
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(
        &mut self,
        sender: mpsc::Sender<String>,
        mut reload: mpsc::Receiver<BlockRefreshMessage>,
    ) {
        self.init().await;
        if sender.send(self.get_status_bar()).await.is_err() {
            // Receiving channel was closed, so there is no point
            // in sending new messages. Quit run.
            return;
        }

        let (schedulers_sender, mut schedulers_receiver) = mpsc::channel::<usize>(8);
        self.blocks
            .iter()
            .map(Block::get_scheduler)
            .flatten()
            .enumerate()
            .for_each(|(i, mut s)| {
                let schedulers_sender = schedulers_sender.clone();
                tokio::spawn(async move {
                    loop {
                        s.tick().await;

                        if schedulers_sender.send(i).await.is_err() {
                            // receiver channel dropped or closed, so we finish as well
                            break;
                        }
                    }
                });
            });
        // drop unused sender
        drop(schedulers_sender);

        let mut reload_finished = false;
        let mut schedulers_finished = false;
        // In this loop we await signals to refresh blocks
        // as well as for custom block reloading using *reload*
        // and we are sending result through *sender* channel.
        loop {
            tokio::select! {
                r = reload.recv(), if !reload_finished => {
                    match r {
                        Some(message) => {
                            let block: &mut Block = match self.get_block_by_name(message.name()) {
                                Some(block) => block,
                                None => {
                                    // For now ignore error and just continue
                                    continue;
                                }
                            };
                            // Ignore errors
                            let _ = block.run(message.mode().clone()).await;

                            if sender.send(self.get_status_bar()).await.is_err() {
                                // Receiving channel was closed, so there is no point
                                // in sending new messages. Quit run.
                                return;
                            }
                        }
                        None => reload_finished = true
                    }
                }
                s = schedulers_receiver.recv(), if !schedulers_finished => {
                    match s {
                        Some(block_index) => {
                            let block: &mut Block = &mut self.blocks[block_index];
                            // Ignore errors
                            let _ = block.run(BlockRunMode::Normal).await;

                            if sender.send(self.get_status_bar()).await.is_err() {
                                // Receiving channel was closed, so there is no point
                                // in sending new messages. Quit run.
                                return;
                            }
                        }
                        None => schedulers_finished = true
                    }
                }
                else => break
            };
        }
    }

    /// Collects `Block`s results and concatenates them into String.
    ///
    /// If `Block`s result is `None` then this block is skipped.
    /// If non of the blocks executed it's command and empty String
    /// is returned.
    fn get_status_bar(&mut self) -> String {
        let mut blocks = self.blocks.iter().map(Block::result).flatten();

        let first = blocks.next();
        if first.is_none() {
            return String::new();
        }

        let mut buffer = match self.buff_size {
            Some(size) => String::with_capacity(size),
            None => String::new(),
        };

        buffer.push_str(first.unwrap());
        blocks.for_each(|r| {
            buffer.push_str(&self.delimiter);
            buffer.push_str(r);
        });

        buffer.shrink_to_fit();
        self.buff_size = Some(buffer.len());
        buffer
    }

    /// Initialises all `Block`s by awaiting completion of [running](Block::run) them.
    async fn init(&mut self) {
        let futures: Vec<_> = self
            .blocks
            .iter_mut()
            .map(|b| b.run(BlockRunMode::Normal))
            .collect();

        let _ = join_all(futures).await;
    }

    fn get_block_by_name(&mut self, name: &str) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|b| b.id() == name)
    }
}

impl Default for StatusBar {
    /// Creates `StatusBar` with no blocks and a single space delimiter.
    fn default() -> Self {
        Self {
            blocks: Vec::default(),
            delimiter: String::from(" "),
            buff_size: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use std::time::SystemTime;
    use tokio::time::{sleep, timeout_at, Duration, Instant};

    fn setup_blocks_for_get_status_bar(
        delimiter: &str,
        data: Vec<Option<&str>>,
        config: Arc<Config>,
    ) -> StatusBar {
        let blocks: Vec<Block> = data
            .iter()
            .map(|x| x.map(|x| x.to_string()))
            .map(|x| {
                let mut block = Block::new("".into(), "".into(), vec![], None, Arc::clone(&config));
                block.set_result(x);
                block
            })
            .collect();

        StatusBar {
            blocks,
            delimiter: String::from(delimiter),
            buff_size: None,
        }
    }

    #[test]
    fn statusbar_get_status_bar() {
        let config = Arc::new(Config::default());
        let mut statusbar = setup_blocks_for_get_status_bar(
            " ",
            vec![Some("A"), Some("B b B"), None, Some("D--")],
            config,
        );
        assert_eq!(String::from("A B b B D--"), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_empty() {
        let mut statusbar = StatusBar::default();
        assert_eq!(String::from(""), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_all_none() {
        let config = Arc::new(Config::default());
        let mut statusbar =
            setup_blocks_for_get_status_bar(" ", vec![None, None, None, None, None], config);
        assert_eq!(String::from(""), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_emojis() {
        let config = Arc::new(Config::default());
        let mut statusbar = setup_blocks_for_get_status_bar(
            " | ",
            vec![Some("🔋 50%"), Some("📅 01/01/2022"), Some("🕒 12:00")],
            config,
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
        let config = Arc::new(Config::default());
        let data = vec![
            Block::new("bat".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("date".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("time".into(), "".into(), vec![], None, Arc::clone(&config)),
        ];
        let cloned_data = data.clone();

        let statusbar = StatusBar::new(data, " ".into());
        assert!(statusbar.is_ok());
        assert_eq!(statusbar.unwrap().blocks, cloned_data);
    }

    #[test]
    fn statusbar_new_err() {
        let config = Arc::new(Config::default());
        let data = vec![
            Block::new("bat".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("date".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("date".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("time".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("time".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("time".into(), "".into(), vec![], None, Arc::clone(&config)),
        ];
        let unique_data = vec![
            Block::new("bat".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("date".into(), "".into(), vec![], None, Arc::clone(&config)),
            Block::new("time".into(), "".into(), vec![], None, Arc::clone(&config)),
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
                delimiter,
                buff_size: None
            }
        );
    }

    #[tokio::test]
    async fn statusbar_init() {
        let config = Arc::new(Config::default());
        // Flag -u sets UTC standard. Since this is what we are comparing
        // this must be set, or this test will fail around midnight.
        let date_block = Block::new(
            "date".into(),
            "date".into(),
            vec!["-u".into(), "+%d/%m/%Y".into()],
            None,
            Arc::clone(&config),
        );
        let info_block = Block::new(
            "info".into(),
            "echo".into(),
            vec!["asyncdwmblocks v1".into()],
            None,
            Arc::clone(&config),
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

    #[test]
    fn get_block_by_name() {
        let config = Arc::new(Config::default());
        let b1 = Block::new(
            "name1".into(),
            "".into(),
            vec![],
            Some(1),
            Arc::clone(&config),
        );
        let b2 = Block::new(
            "name2".into(),
            "".into(),
            vec![],
            Some(2),
            Arc::clone(&config),
        );

        let mut status_bar = StatusBar::new(vec![b1, b2], " ".into()).unwrap();

        let b1 = status_bar.get_block_by_name("name1");
        assert!(b1.is_some());
        let b1 = b1.unwrap();
        assert_eq!(b1.get_interval(), Some(Duration::from_secs(1)));

        let b2 = status_bar.get_block_by_name("name2");
        assert!(b2.is_some());
        let b2 = b2.unwrap();
        assert_eq!(b2.get_interval(), Some(Duration::from_secs(2)));

        let none = status_bar.get_block_by_name("non_existing_id");
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn run_intervals() {
        let config = Arc::new(Config::default());
        let b = Block::new(
            "epoch".into(),
            "date".into(),
            vec!["+%s".into()],
            Some(1),
            Arc::clone(&config),
        );
        let mut status_bar = StatusBar::new(vec![b], "".into()).unwrap();

        let (result_sender, mut result_receiver) = mpsc::channel(8);
        let (_, reload_receiver) = mpsc::channel(8);

        tokio::spawn(async move {
            status_bar.run(result_sender, reload_receiver).await;
        });

        // initial run
        let _ = result_receiver.recv().await;

        let result = timeout_at(
            Instant::now() + Duration::from_millis(10),
            result_receiver.recv(),
        )
        .await;

        assert!(result.is_err());

        let result = timeout_at(
            Instant::now() + Duration::from_secs(1) + Duration::from_millis(10),
            result_receiver.recv(),
        )
        .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn run_intervals_reload() {
        let config = Arc::new(Config::default());
        let b = Block::new(
            "epoch".into(),
            "date".into(),
            vec!["+%s".into()],
            None,
            Arc::clone(&config),
        );
        let mut status_bar = StatusBar::new(vec![b], "".into()).unwrap();

        let (result_sender, mut result_receiver) = mpsc::channel(8);
        let (reload_sender, reload_receiver) = mpsc::channel(8);

        tokio::spawn(async move {
            status_bar.run(result_sender, reload_receiver).await;
        });

        // initial run
        let _ = result_receiver.recv().await;

        let timeout = timeout_at(
            Instant::now() + Duration::from_millis(10),
            result_receiver.recv(),
        )
        .await;
        assert!(timeout.is_err());

        reload_sender
            .send(BlockRefreshMessage::new(
                "epoch".into(),
                BlockRunMode::Normal,
            ))
            .await
            .unwrap();
        let timeout = timeout_at(
            Instant::now() + Duration::from_millis(10),
            result_receiver.recv(),
        )
        .await;
        assert!(timeout.is_ok());

        // test closing channels
        drop(reload_sender);
        let result = result_receiver.recv().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn run_intervals_channel_on_task() {
        let config = Arc::new(Config::default());
        let b = Block::new(
            "epoch".into(),
            "date".into(),
            vec!["+%s".into()],
            None,
            Arc::clone(&config),
        );
        let mut status_bar = StatusBar::new(vec![b], "".into()).unwrap();

        let (result_sender, mut result_receiver) = mpsc::channel(8);
        let (reload_sender, reload_receiver) = mpsc::channel(8);

        tokio::spawn(async move {
            // initial run
            let _ = result_receiver.recv().await;

            let timeout = timeout_at(
                Instant::now() + Duration::from_millis(10),
                result_receiver.recv(),
            )
            .await;
            assert!(timeout.is_err());

            reload_sender
                .send(BlockRefreshMessage::new(
                    "epoch".into(),
                    BlockRunMode::Normal,
                ))
                .await
                .unwrap();
            let timeout = timeout_at(
                Instant::now() + Duration::from_millis(10),
                result_receiver.recv(),
            )
            .await;
            assert!(timeout.is_ok());
        });

        let timeout = timeout_at(
            Instant::now() + Duration::from_millis(30),
            status_bar.run(result_sender, reload_receiver),
        )
        .await;
        assert!(timeout.is_ok());
    }

    #[tokio::test]
    async fn run_test_asynchronicity() {
        // XXX: ~40 seems to be upper throughput limit. Since it is more
        // than enough for real world use I will leave it as it is for now.
        // Maybe later I will try to figure out if there is something I am
        // doing wrong and try to fix/optimize it.
        const NUM: usize = 40;

        let config = Arc::new(Config::default());
        let blocks: Vec<Block> = (0..NUM)
            .map(|i| {
                Block::new(
                    format!("echo_{}", i),
                    "echo".into(),
                    vec![format!("{}", i)],
                    Some(1),
                    Arc::clone(&config),
                )
            })
            .collect();
        let mut status_bar = StatusBar::new(blocks, " ".into()).unwrap();

        let (result_sender, mut result_receiver) = mpsc::channel(2 * NUM);
        let (_, reload_receiver) = mpsc::channel(8);

        tokio::spawn(async move {
            status_bar.run(result_sender, reload_receiver).await;
        });

        // initial run
        let _ = result_receiver.recv().await;

        sleep(Duration::from_secs(1) + Duration::from_millis(100)).await;

        assert_eq!(
            NUM,
            (0..)
                .map(|_| result_receiver.try_recv())
                .take_while(|r| r.is_ok())
                .count()
        );
    }
}
