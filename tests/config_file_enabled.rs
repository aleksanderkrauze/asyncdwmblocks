#![cfg(feature = "config-file")]
#![allow(unused_imports)]

use std::env;
use std::path::PathBuf;

use pretty_assertions::assert_eq;
use rusty_fork::rusty_fork_test;

use asyncdwmblocks::config::{self, Config};
#[cfg(feature = "ipc")]
use asyncdwmblocks::ipc::ServerType;

#[tokio::test]
async fn load_configuration_no_ipc() {
    let config = Config::load_from_file("./tests/assets/config_no_ipc.yaml")
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
}

#[cfg(feature = "tcp")]
#[tokio::test]
async fn load_configuration_tcp() {
    let config = Config::load_from_file("./tests/assets/config_tcp.yaml")
        .await
        .unwrap();

    assert_eq!(config.ipc.server_type, ServerType::Tcp);
    assert_eq!(config.ipc.tcp.port, 44005);
}

#[cfg(feature = "uds")]
#[tokio::test]
async fn load_configuration_uds() {
    let config = Config::load_from_file("./tests/assets/config_uds.yaml")
        .await
        .unwrap();

    assert_eq!(config.ipc.server_type, ServerType::UnixDomainSocket);
    assert_eq!(
        config.ipc.uds.addr,
        PathBuf::from("/home/username/.local/share/asyncdwmblocks/asyncdwmblocks.socket")
    );
}

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
