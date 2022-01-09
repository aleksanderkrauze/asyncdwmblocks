#![allow(unused_imports)]

use std::env;

use rusty_fork::rusty_fork_test;

use asyncdwmblocks::config::Config;
#[cfg(feature = "ipc")]
use asyncdwmblocks::ipc::ServerType;

#[cfg(feature = "config-file")]
#[tokio::test]
async fn load_full_configuration() {
    let config = Config::load_from_file("./tests/assets/full_config.yaml")
        .await
        .unwrap();

    assert_eq!(config.button_env_variable, String::from("BTN"));
    #[cfg(feature = "tcp")]
    {
        assert_eq!(config.server_type, ServerType::Tcp);
        assert_eq!(config.tcp_port, 44005);
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

            assert_eq!(config.button_env_variable, String::from("1"));
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

            assert_eq!(config.button_env_variable, String::from("2"));
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
