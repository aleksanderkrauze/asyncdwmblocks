#![cfg(not(feature = "config-file"))]

use asyncdwmblocks::config::Config;

#[tokio::test]
async fn get_config_returned_defaut_config_on_config_file_feature_disabled() {
    let config = Config::get_config().await.unwrap();
    assert_eq!(config, Config::default());
}
