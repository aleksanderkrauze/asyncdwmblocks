use std::sync::Arc;

use crate::server::ServerType;

#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub button_env_variable: String,
    pub tcp_port: u16,
    pub server_type: ServerType,
}

impl Config {
    /// Wraps [Config] into [Arc].
    ///
    /// Because many structs contain `Arc<Config>` this method allows to easily
    /// wrap Config into Arc without need to import Arc and calling ugly `Arc::new`.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::config::Config;
    /// use std::sync::Arc;
    /// # fn main() {
    /// let config = Config::default().arc();
    /// assert_eq!(config, Arc::new(Config::default()));
    /// # }
    /// ```
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            button_env_variable: String::from("BUTTON"),
            tcp_port: 44000,
            server_type: ServerType::Tcp,
        }
    }
}
