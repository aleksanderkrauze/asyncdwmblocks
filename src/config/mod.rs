//! This module defines struct [Config] used to set behaviour of
//! many parts of this library and statusbar executables.
//!
//! Apart from `Config` this module defines also many structs used by
//! `Config` "internally". It also defines [ConfigLoadError], returned
//! when loading configuration from file fails.
//!
//! There are three ways in which `Config` can be created:
//!  - manually
//!  - from default configuration
//!  - from file
//!
//!  The blessed way is to use [Config::get_config], which if asyncdwmblocks was
//!  compiled with `config-file` feature will try to load configuration from files
//!  (see it's documentation) and fall back to default Config if files were not found,
//!  or provide just default configuration when feature `config-file` was not enabled.
//!
//!  Deafault implementations of this config structs is in `defaults` submodule,
//!  that is located in `src/config/defaults.rs`. If you wish to use asyncdwmblocks
//!  without configuration files (in true suckless spirit), you should go there
//!  to set your configuration. The most important configuration will probably be setting
//!  blocks, which can be done in `default_statusbar_blocks` function.

// Allow unused imports, so that if features do not require
// some import, rust will not complain. Thanks to this we do
// not have to prefix each such import with #[cfg()] flag.
#![allow(unused_imports)]

mod defaults;

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "config-file")]
use serde::Deserialize;
#[cfg(feature = "config-file")]
use tokio::fs;

#[cfg(feature = "ipc")]
use crate::ipc::ServerType;

/// Error returned when loading Config from file failed.
#[cfg(feature = "config-file")]
#[derive(Debug)]
pub enum ConfigLoadError {
    /// IO error ocurred.
    IO(std::io::Error),
    /// Loaded data couldn't be deserialized into Config.
    DeserializeError(serde_yaml::Error),
}

#[cfg(feature = "config-file")]
impl From<std::io::Error> for ConfigLoadError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

#[cfg(feature = "config-file")]
impl From<serde_yaml::Error> for ConfigLoadError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::DeserializeError(err)
    }
}

#[cfg(feature = "config-file")]
impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::IO(err) => format!("IO error: {}", err),
            Self::DeserializeError(err) => format!("Deserialization error: {}", err),
        };

        write!(f, "{}", msg)
    }
}

#[cfg(feature = "config-file")]
impl Error for ConfigLoadError {}

/// StatusBar's block representation.
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
pub struct ConfigStatusBarBlock {
    /// Block's name (id)
    pub name: String,
    /// Command to run
    pub command: String,
    /// Command's args
    #[cfg_attr(feature = "config-file", serde(default))]
    pub args: Vec<String>,
    /// Refresh interval
    #[cfg_attr(feature = "config-file", serde(default))]
    pub interval: Option<u64>,
}

/// Configuration for [StatusBar](crate::statusbar::StatusBar).
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct ConfigStatusBar {
    /// StatusBar's delimiter.
    pub delimiter: String,
    /// List of StatusBar Blocks.
    pub blocks: Vec<ConfigStatusBarBlock>,
}

/// Configuration for [Blocks](crate::block::Block).
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct ConfigBlock {
    /// Environment variable used to comunicate that block was clicked.
    pub clicked_env_variable: String,
}

/// Configuration of Tcp Server/Notifier.
#[cfg(feature = "tcp")]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct ConfigIpcTcp {
    /// Port on which TCP Server/Notier listens on/connects to.
    pub port: u16,
}

/// Configuration of Unix domain socket Server/Notifier.
#[cfg(feature = "uds")]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct ConfigIpcUnixDomainSocket {
    /// Address on which Unix domain socket Server/Notier listens on/connects to.
    pub addr: PathBuf,
}

/// Configuration for IPC (inter progess cominiucation).
#[cfg(feature = "ipc")]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct ConfigIpc {
    /// Which type of IPC should be used.
    #[cfg_attr(feature = "config-file", serde(rename = "type"))]
    pub server_type: ServerType,
    /// Configuration of TCP Server/Notifier.
    #[cfg(feature = "tcp")]
    pub tcp: ConfigIpcTcp,
    /// Configuration of Unix domain socket Server/Notifier.
    #[cfg(feature = "uds")]
    pub uds: ConfigIpcUnixDomainSocket,
}

/// Main configuration struct.
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "config-file", derive(Deserialize))]
#[cfg_attr(feature = "config-file", serde(default))]
pub struct Config {
    /// Configuration of [`StatusBar`](crate::statusbar::StatusBar).
    pub statusbar: ConfigStatusBar,
    /// Configuration of [`Blocks`](crate::block::Block).
    pub block: ConfigBlock,
    /// Configuration of IPC (inter process comunication).
    #[cfg(feature = "ipc")]
    pub ipc: ConfigIpc,
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

    /// Tries to load `Config` from file. If config file can't be found
    /// or asyncdwmblocks was compiled without `config-file` feature,
    /// then [default](Default) `Config` is returned.
    ///
    /// This function tries to locate config file in following locations
    /// (and following order):
    /// - `$XDG_CONFIG_HOME/asyncdwmblocks/config.yaml`
    /// - `$HOME/.config/asyncdwmblocks/config.yaml`
    pub async fn get_config() -> Result<Config, Box<dyn Error>> {
        #[cfg(feature = "config-file")]
        {
            // check $XDG_CONFIG_HOME/asyncdwmblocks/config.yaml
            if let Some(var) = std::env::var_os("XDG_CONFIG_HOME") {
                let mut path = std::path::PathBuf::from(var);
                path.push("asyncdwmblocks/config.yaml");

                // Metadata returned Ok(), so file exists
                if fs::metadata(&path).await.is_ok() {
                    return Config::load_from_file(&path)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn Error>);
                }
            }

            // check $HOME/.config/asyncdwmblocks/config.yaml
            if let Some(var) = std::env::var_os("HOME") {
                let mut path = std::path::PathBuf::from(var);
                path.push(".config/asyncdwmblocks/config.yaml");

                // Metadata returned Ok(), so file exists
                if fs::metadata(&path).await.is_ok() {
                    return Config::load_from_file(&path)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn Error>);
                }
            }
        }
        // return default
        Ok(Config::default())
    }

    /// Tries to load configuration from given file.
    ///
    /// It can fail int the event of an IO error, or deserialization error.
    #[cfg(feature = "config-file")]
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigLoadError> {
        let file_data = fs::read(path).await?;
        let config = serde_yaml::from_slice(file_data.as_slice())?;

        Ok(config)
    }
}
