//! This module defines struct [Config] used to set behaviour of
//! many parts of this library and statusbar executables.

#[cfg(feature = "config-file")]
mod config_file;
#[cfg(feature = "config-file")]
pub use config_file::ConfigLoadError;

use std::sync::Arc;

#[cfg(feature = "ipc")]
use crate::ipc::ServerType;

/// Main configuration struct.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    /// Name of the environment variable that is set for running
    /// block's process when this block was "clicked".
    /// Defaults to `$BUTTON`.
    pub button_env_variable: String,
    /// TCP port that asyncdwmblocks listens on for refreshing blocks
    /// on demand. Used when [ServerType] is TCP. Defaults to 44000.
    #[cfg(feature = "tcp")]
    pub tcp_port: u16,
    /// Type of server (and notifier) for communication between processes.
    #[cfg(feature = "ipc")]
    pub server_type: ServerType,
}

impl Config {
    /// Wraps [Config] in [Arc].
    ///
    /// Because many structs contain `Arc<Config>` this method allows to easily
    /// wrap Config into Arc without need to import Arc and calling ugly `Arc::new`.
    ///
    /// # Example
    /// ```
    /// use asyncdwmblocks::config::Config;
    /// use std::sync::Arc;
    ///
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
        #[cfg(feature = "ipc")]
        let server_type = {
            #[cfg(feature = "tcp")]
            let server_type = ServerType::Tcp;

            server_type
        };

        Self {
            button_env_variable: String::from("BUTTON"),
            #[cfg(feature = "tcp")]
            tcp_port: 44000,
            #[cfg(feature = "ipc")]
            server_type,
        }
    }
}
