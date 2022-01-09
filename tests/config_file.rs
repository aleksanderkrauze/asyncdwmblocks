use asyncdwmblocks::config::Config;

#[tokio::test]
async fn load_full_configuration() {
    let config = Config::load_from_file("./tests/assets/full_config.yaml")
        .await
        .unwrap();

    assert_eq!(config.button_env_variable, String::from("BTN"));
    #[cfg(feature = "tcp")]
    {
        assert_eq!(config.server_type, asyncdwmblocks::ipc::ServerType::Tcp);
        assert_eq!(config.tcp_port, 44005);
    }
}
