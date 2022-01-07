use asyncdwmblocks::block::{Block, BlockRunMode};
use asyncdwmblocks::config::Config;

#[tokio::test]
async fn run_mode_button() {
    let config = Config::default().arc();
    let mut block = Block::new(
        "button".into(),
        "./tests/assets/button.sh".into(),
        vec![],
        None,
        config,
    );

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
        button_env_variable: String::from("BTN"),
        ..Config::default()
    }
    .arc();
    let mut block = Block::new(
        "button".into(),
        "./tests/assets/button_btn.sh".into(),
        vec![],
        None,
        config,
    );

    block.run(BlockRunMode::Button(1)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("1")));

    block.run(BlockRunMode::Normal).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("")));

    block.run(BlockRunMode::Button(3)).await.unwrap();
    assert_eq!(block.result(), Some(&String::from("3")));
}
