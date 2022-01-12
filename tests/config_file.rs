#![allow(unused_imports)]

use std::env;

use pretty_assertions::assert_eq;
use rusty_fork::rusty_fork_test;

use asyncdwmblocks::config::{self, Config};
#[cfg(feature = "ipc")]
use asyncdwmblocks::ipc::ServerType;

#[cfg(feature = "config-file")]
#[tokio::test]
async fn load_full_configuration() {
    let config = Config::load_from_file("./tests/assets/full_config.yaml")
        .await
        .unwrap();

    assert_eq!(config.statusbar.delimiter, String::from(" | "));
    assert_eq!(
        config.statusbar.blocks,
        vec![
            config::ConfigStatusBarBlock {
                name: String::from("volume"),
                command: String::from("my_volume_script.sh"),
                args: vec![],
                interval: None
            },
            config::ConfigStatusBarBlock {
                name: String::from("battery"),
                command: String::from("my_battery_script.sh"),
                args: vec![],
                interval: Some(60)
            },
            config::ConfigStatusBarBlock {
                name: String::from("date"),
                command: String::from("my_datetime_script.sh"),
                args: vec![String::from("--my-arg 5"), String::from("today")],
                interval: Some(1)
            },
        ]
    );

    assert_eq!(config.block.clicked_env_variable, String::from("BTN"));

    #[cfg(feature = "tcp")]
    {
        assert_eq!(config.ipc.server_type, ServerType::Tcp);
        assert_eq!(config.ipc.tcp.port, 44005);
    }
}

#[cfg(feature = "config-file")]
rusty_fork_test! {
    #[test]
    fn get_config_xdg() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            env::set_var("XDG_CONFIG_HOME", "./tests/assets/config_autoload/1");
            let config = Config::get_config().await.unwrap();

            assert_eq!(config.block.clicked_env_variable, String::from("1"));
        });
    }
}

#[cfg(feature = "config-file")]
rusty_fork_test! {
    #[test]
    fn get_config_home() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

        rt.block_on(async {
            env::remove_var("XDG_CONFIG_HOME");
            env::set_var("HOME", "./tests/assets/config_autoload/2");
            let config = Config::get_config().await.unwrap();

            assert_eq!(config.block.clicked_env_variable, String::from("2"));
        });
    }
}

#[cfg(feature = "config-file")]
rusty_fork_test! {
    #[test]
    fn get_config_default() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

        rt.block_on(async {
            env::remove_var("XDG_CONFIG_HOME");
            env::remove_var("HOME");
            let config = Config::get_config().await.unwrap();

            assert_eq!(config, Config::default());
        });
    }
}

#[cfg(not(feature = "config-file"))]
#[tokio::test]
async fn get_config_returned_defaut_config_on_config_file_feature_disabled() {
    let config = Config::get_config().await.unwrap();
    assert_eq!(config, Config::default());
}
