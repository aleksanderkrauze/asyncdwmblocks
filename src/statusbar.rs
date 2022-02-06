//! This module defines [StatusBar] and it's errors.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use futures::future::join_all;
use indexmap::IndexMap;
use tokio::sync::mpsc;

use crate::block::{Block, BlockRunMode};
use crate::config::Config;

/// [Block] held by [StatusBar].
#[derive(Debug, PartialEq, Clone)]
pub struct StatusBarBlock {
    /// Block's name
    pub name: String,
    /// Block
    pub block: Block,
}

/// Message passed to [StatusBar] informing it which block should
/// be refreshed and how.
#[derive(Debug, PartialEq, Clone)]
pub struct BlockRefreshMessage {
    /// Name (id) of a block that should be refreshed
    pub(crate) name: String,
    /// In which mode should this block be refreshed
    pub(crate) mode: BlockRunMode,
}

impl BlockRefreshMessage {
    /// Creates new `BlockRefreshMessage`.
    pub fn new(name: String, mode: BlockRunMode) -> Self {
        Self { name, mode }
    }
}

/// Error that represents failure to create StatusBar.
#[derive(Debug, PartialEq, Clone)]
pub enum StatusBarCreationError {
    /// Multiple blocks had the same name
    BlockIdError(String),
}

impl fmt::Display for StatusBarCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::BlockIdError(msg) => format!("Each block id should be unique\n\n{}", msg),
        };

        write!(f, "{}", msg)
    }
}

impl Error for StatusBarCreationError {}

/// This struct represents a status bar.
///
/// `StatusBar` is a collection of `Block`s that can refresh them at
/// their interval and also listen to incoming requests to refresh
/// specific block. It reads delimiter from config, that is put
/// between each pair of adjacent blocks.
///
/// `StatusBar` can be created either manually by calling [new](StatusBar::new)
/// or [try_from](StatusBar::try_from<Config>) [`Config`] (which is preferred way).
#[derive(Debug, PartialEq, Clone)]
pub struct StatusBar {
    blocks: IndexMap<String, Block>,
    config: Arc<Config>,
    buff_size: Option<usize>,
}

impl StatusBar {
    /// Creates new `StatusBar` from vector of [`StatusBarBlock`]s.
    ///
    /// Will return error if some blocks have the same name.
    ///
    /// # Example
    /// ```no_run
    /// use std::sync::Arc;
    /// use asyncdwmblocks::block::Block;
    /// use asyncdwmblocks::statusbar::{StatusBar, StatusBarBlock};
    /// use asyncdwmblocks::config::Config;
    ///
    /// # fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::default().arc();
    /// let battery = Block::new("my_battery_script".into(), vec![], Some(60), Arc::clone(&config));
    /// let datetime = Block::new("my_datetime_script".into(), vec![], Some(60), Arc::clone(&config));
    /// let info = Block::new("echo".into(), vec!["asyncdwmblocks".into()], None, Arc::clone(&config));
    ///
    /// let blocks = vec![
    ///     StatusBarBlock { name: "battery".to_string(), block: battery },
    ///     StatusBarBlock { name: "datetime".to_string(), block: datetime },
    ///     StatusBarBlock { name: "info".to_string(), block: info },
    /// ];
    /// let statusbar = StatusBar::new(blocks, config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        blocks: Vec<StatusBarBlock>,
        config: Arc<Config>,
    ) -> Result<Self, StatusBarCreationError> {
        let mut blocks_map = IndexMap::with_capacity(blocks.len());
        let mut err_map = IndexMap::<String, usize>::new();

        for StatusBarBlock { name, block } in blocks {
            if !blocks_map.contains_key(&name) {
                blocks_map.insert(name, block);
            } else {
                *err_map.entry(name).or_insert(1) += 1;
            }
        }

        if !err_map.is_empty() {
            let mut err_msg = String::new();
            for (name, num) in err_map {
                err_msg.push_str(&format!("Name: `{}` occurs multiple ({}) times", name, num));
            }
            Err(StatusBarCreationError::BlockIdError(err_msg))
        } else {
            Ok(Self {
                blocks: blocks_map,
                config,
                buff_size: None,
            })
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
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::mpsc;
    /// use asyncdwmblocks::block::Block;
    /// use asyncdwmblocks::statusbar::{StatusBar, StatusBarBlock};
    /// use asyncdwmblocks::config::Config;
    ///
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::default().arc();
    /// let b = Block::new("date".into(), vec![], Some(60), Arc::clone(&config));
    /// let mut status_bar = StatusBar::new(
    ///     vec![StatusBarBlock { name: "date_block".to_string(), block: b } ],
    ///     config
    /// )?;
    ///
    /// let (result_sender, mut result_receiver) = mpsc::channel(8);
    /// let (reload_sender, reload_receiver) = mpsc::channel(8);
    ///
    /// tokio::spawn(async move {
    ///     status_bar.run(result_sender, reload_receiver).await;
    /// });
    ///
    /// while let Some(_) = result_receiver.recv().await {
    ///     // do stuff
    ///     # break;
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

        let (schedulers_sender, mut schedulers_receiver) = mpsc::channel(8);
        for (index, block) in self.blocks.values().enumerate() {
            if let Some(mut scheduler) = block.get_scheduler() {
                let schedulers_sender = schedulers_sender.clone();
                tokio::spawn(async move {
                    loop {
                        scheduler.tick().await;

                        if schedulers_sender.send(index).await.is_err() {
                            // receiver channel dropped or closed, so we finish as well
                            break;
                        }
                    }
                });
            }
        }
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
                            let block: &mut Block = match self.get_block_by_name_mut(&message.name) {
                                Some(block) => block,
                                None => {
                                    // For now ignore error and just continue
                                    continue;
                                }
                            };
                            // TODO: crash on internal error
                            // Ignore errors
                            let _ = block.run(message.mode.clone()).await;

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
                        Some(index) => {
                            // It is safe to index into self.blocks, because this index was created
                            // while enumerating it's values.
                            let block = &mut self.blocks[index];
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
        let mut blocks = self
            .blocks
            .iter()
            .map(|(_, block)| block)
            .map(Block::result)
            .flatten();

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
            buffer.push_str(&self.config.statusbar.delimiter);
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
            .map(|(_, block)| block)
            .map(|b| b.run(BlockRunMode::Normal))
            .collect();

        let _ = join_all(futures).await;
    }

    fn get_block_by_name_mut(&mut self, name: &str) -> Option<&mut Block> {
        self.blocks.get_mut(name)
    }
}

impl TryFrom<Arc<Config>> for StatusBar {
    type Error = StatusBarCreationError;
    fn try_from(config: Arc<Config>) -> Result<Self, Self::Error> {
        let blocks = config
            .statusbar
            .blocks
            .iter()
            .map(|b| StatusBarBlock {
                name: b.name.clone(),
                block: Block::new(
                    b.command.clone(),
                    b.args.clone(),
                    b.interval,
                    Arc::clone(&config),
                ),
            })
            .collect();
        Self::new(blocks, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use chrono::{DateTime, Utc};
    use std::time::SystemTime;
    use tokio::time::{sleep, timeout_at, Duration, Instant};

    use pretty_assertions::assert_eq;

    fn setup_blocks_for_get_status_bar(data: Vec<Option<&str>>, config: Arc<Config>) -> StatusBar {
        let blocks: IndexMap<String, Block> = data
            .iter()
            .map(|x| x.map(|x| x.to_string()))
            .map(|x| {
                let mut block = Block::new("".into(), vec![], None, Arc::clone(&config));
                block.set_result(x);
                block
            })
            .enumerate()
            .map(|(i, b)| (format!("id_{}", i), b))
            .collect();

        StatusBar {
            blocks,
            config,
            buff_size: None,
        }
    }

    #[test]
    fn statusbar_get_status_bar() {
        let config = Config {
            statusbar: config::ConfigStatusBar {
                delimiter: " ".into(),
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();
        let mut statusbar = setup_blocks_for_get_status_bar(
            vec![Some("A"), Some("B b B"), None, Some("D--")],
            config,
        );
        assert_eq!(String::from("A B b B D--"), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_all_none() {
        let config = Config {
            statusbar: config::ConfigStatusBar {
                delimiter: " ".into(),
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();
        let mut statusbar =
            setup_blocks_for_get_status_bar(vec![None, None, None, None, None], config);
        assert_eq!(String::from(""), statusbar.get_status_bar());
    }

    #[test]
    fn statusbar_get_status_bar_emojis() {
        let config = Config {
            statusbar: config::ConfigStatusBar {
                delimiter: " | ".into(),
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();
        let mut statusbar = setup_blocks_for_get_status_bar(
            vec![Some("🔋 50%"), Some("📅 01/01/2022"), Some("🕒 12:00")],
            config,
        );
        assert_eq!(
            String::from("🔋 50% | 📅 01/01/2022 | 🕒 12:00"),
            statusbar.get_status_bar()
        );
    }

    #[tokio::test]
    async fn statusbar_init() {
        let config = Config {
            statusbar: config::ConfigStatusBar {
                delimiter: " | ".into(),
                ..Default::default()
            },
            ..Default::default()
        }
        .arc();
        // Flag -u sets UTC standard. Since this is what we are comparing
        // this must be set, or this test will fail around midnight.
        let date_block = Block::new(
            "date".into(),
            vec!["-u".into(), "+%d/%m/%Y".into()],
            None,
            Arc::clone(&config),
        );
        let info_block = Block::new(
            "echo".into(),
            vec!["asyncdwmblocks v1".into()],
            None,
            Arc::clone(&config),
        );

        let current_date: DateTime<Utc> = DateTime::from(SystemTime::now());
        let current_date = current_date.format("%d/%m/%Y").to_string();

        let mut statusbar = StatusBar::new(
            vec![
                StatusBarBlock {
                    name: "date".into(),
                    block: date_block,
                },
                StatusBarBlock {
                    name: "info".into(),
                    block: info_block,
                },
            ],
            config,
        )
        .unwrap();
        statusbar.init().await;

        assert_eq!(
            statusbar.get_status_bar(),
            format!("{} | asyncdwmblocks v1", current_date)
        );
    }

    #[test]
    fn get_block_by_name() {
        let config = Config::default().arc();
        let b1 = Block::new("".into(), vec![], Some(1), Arc::clone(&config));
        let b2 = Block::new("".into(), vec![], Some(2), Arc::clone(&config));

        let mut status_bar = StatusBar::new(
            vec![
                StatusBarBlock {
                    name: "name1".into(),
                    block: b1,
                },
                StatusBarBlock {
                    name: "name2".into(),
                    block: b2,
                },
            ],
            config,
        )
        .unwrap();

        let b1 = status_bar.get_block_by_name_mut("name1");
        assert!(b1.is_some());
        let b1 = b1.unwrap();
        assert_eq!(b1.get_interval(), Some(Duration::from_secs(1)));

        let b2 = status_bar.get_block_by_name_mut("name2");
        assert!(b2.is_some());
        let b2 = b2.unwrap();
        assert_eq!(b2.get_interval(), Some(Duration::from_secs(2)));

        let none = status_bar.get_block_by_name_mut("non_existing_id");
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn run_intervals() {
        let config = Config::default().arc();
        let b = Block::new(
            "date".into(),
            vec!["+%s".into()],
            Some(1),
            Arc::clone(&config),
        );
        let mut status_bar = StatusBar::new(
            vec![StatusBarBlock {
                name: "epoch".into(),
                block: b,
            }],
            config,
        )
        .unwrap();

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
        let config = Config::default().arc();
        let b = Block::new("date".into(), vec!["+%s".into()], None, Arc::clone(&config));
        let mut status_bar = StatusBar::new(
            vec![StatusBarBlock {
                name: "epoch".into(),
                block: b,
            }],
            config,
        )
        .unwrap();

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
        let config = Config::default().arc();
        let b = Block::new("date".into(), vec!["+%s".into()], None, Arc::clone(&config));
        let mut status_bar = StatusBar::new(
            vec![StatusBarBlock {
                name: "epoch".into(),
                block: b,
            }],
            config,
        )
        .unwrap();

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

        let config = Config::default().arc();
        let blocks: Vec<StatusBarBlock> = (0..NUM)
            .map(|i| StatusBarBlock {
                name: format!("echo_{}", i),
                block: Block::new(
                    "echo".into(),
                    vec![i.to_string()],
                    Some(1),
                    Arc::clone(&config),
                ),
            })
            .collect();
        let mut status_bar = StatusBar::new(blocks, config).unwrap();

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

    #[tokio::test]
    async fn statusbar_blocks_from_config() {
        let blocks = vec![
            config::ConfigStatusBarBlock {
                name: String::from("block1"),
                command: String::from("echo"),
                args: vec![String::from("I")],
                interval: None,
            },
            config::ConfigStatusBarBlock {
                name: String::from("block2"),
                command: String::from("echo"),
                args: vec![String::from("🦀!")],
                interval: None,
            },
        ];
        let config = Config {
            statusbar: config::ConfigStatusBar {
                blocks,
                delimiter: String::from(" ❤️ "),
            },
            ..Default::default()
        }
        .arc();

        let mut statusbar = StatusBar::try_from(config).unwrap();
        statusbar.init().await;

        assert_eq!(statusbar.get_status_bar(), String::from("I ❤️ 🦀!"));
    }

    #[test]
    fn statusbar_multiple_ids_error() {
        let config = Config::default().arc();
        let blocks = vec![
            StatusBarBlock {
                name: "A".into(),
                block: Block::new(String::from("1"), vec![], None, Arc::clone(&config)),
            },
            StatusBarBlock {
                name: "B".into(),
                block: Block::new(String::from("2"), vec![], None, Arc::clone(&config)),
            },
            StatusBarBlock {
                name: "B".into(),
                block: Block::new(String::from("3"), vec![], None, Arc::clone(&config)),
            },
            StatusBarBlock {
                name: "A".into(),
                block: Block::new(String::from("4"), vec![], None, Arc::clone(&config)),
            },
            StatusBarBlock {
                name: "C".into(),
                block: Block::new(String::from("5"), vec![], None, Arc::clone(&config)),
            },
        ];

        let statusbar = StatusBar::new(blocks, config);

        assert!(statusbar.is_err());
    }
}
