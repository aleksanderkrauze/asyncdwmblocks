use asyncdwmblocks::block::{Block, BlockRunMode};

#[tokio::test]
async fn run_mode_button() {
    let mut block = Block::new(
        "button".into(),
        "./tests/assets/button.sh".into(),
        vec![],
        None,
    );

    block.run(BlockRunMode::Button(1)).await.unwrap();
    assert_eq!(block.result(), &Some("1".into()));

    block.run(BlockRunMode::Normal).await.unwrap();
    assert_eq!(block.result(), &Some("".into()));

    block.run(BlockRunMode::Button(3)).await.unwrap();
    assert_eq!(block.result(), &Some("3".into()));
}
