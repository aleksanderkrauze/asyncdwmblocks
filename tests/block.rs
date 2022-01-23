use asyncdwmblocks::block::{Block, BlockRunMode};
use asyncdwmblocks::config::{self, Config};

#[tokio::test]
async fn run_mode_button() {
    let config = Config::default().arc();
    let mut block = Block::new("./tests/assets/button.sh".into(), vec![], None, config);

    block.run(BlockRunMode::Button(1)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("1")));

    block.run(BlockRunMode::Normal).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("")));

    block.run(BlockRunMode::Button(3)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("3")));
}

#[tokio::test]
async fn run_mode_button_changed_env_variable() {
    let config = Config {
        block: config::ConfigBlock {
            clicked_env_variable: String::from("BTN"),
        },
        ..Config::default()
    }
    .arc();
    let mut block = Block::new("./tests/assets/button_btn.sh".into(), vec![], None, config);

    block.run(BlockRunMode::Button(1)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("1")));

    block.run(BlockRunMode::Normal).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("")));

    block.run(BlockRunMode::Button(3)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("3")));
}

#[tokio::test]
async fn filter_out_null_chars() {
    let config = Config::default().arc();
    let mut block = Block::new(
        "./tests/assets/echo_null_char.sh".into(),
        vec![],
        None,
        config,
    );

    block.run(BlockRunMode::Normal).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("ABC123")));
}
