#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub button_env_variable: String,
    pub tcp_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            button_env_variable: String::from("BUTTON"),
            tcp_port: 44000,
        }
    }
}
