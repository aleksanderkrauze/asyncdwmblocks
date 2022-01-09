//! This module defines struct [Config] used to set behaviour of
//! many parts of this library and statusbar executables.

#![cfg_attr(not(feature = "config-file"), allow(unused_imports))]

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "ipc")]
use crate::ipc::ServerType;

#[cfg(feature = "config-file")]
use tokio::fs;
#[cfg(feature = "config-file")]
use yaml_rust::{scanner::ScanError, YamlLoader};

/// This enum represents an error returned whilst loading
/// configuration from config file.
#[cfg(feature = "config-file")]
#[derive(Debug)]
pub enum ConfigLoadError {
    /// IO error that occurred while opening/reading a file.
    IO(std::io::Error),
    /// File couldn't be decoded as a valid UTF-8 document.
    UTF8,
    /// File couldn't be parsed as a valid YAML document.
    YamlParse(ScanError),
    /// File was parsed as a valid YAML document, but
    /// some field has wrong type.
    Syntax(String),
    /// Some option is wrongly set or doesn't make sense.
    ConfigError(String),
}

#[cfg(feature = "config-file")]
impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::IO(err) => format!("IO error: {}", err),
            Self::UTF8 => "Config file is not a valid UTF-8 document.".to_string(),
            Self::YamlParse(err) => {
                format!("Coulnd't parse config file as a valid yaml file: {}", err)
            }
            Self::Syntax(err) => format!("Wrong syntax: {}", err),
            Self::ConfigError(err) => format!("Error: {}", err),
        };

        write!(f, "{}", msg)
    }
}

#[cfg(feature = "config-file")]
impl From<std::io::Error> for ConfigLoadError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

#[cfg(feature = "config-file")]
impl From<std::string::FromUtf8Error> for ConfigLoadError {
    fn from(_err: std::string::FromUtf8Error) -> Self {
        Self::UTF8
    }
}

#[cfg(feature = "config-file")]
impl From<ScanError> for ConfigLoadError {
    fn from(err: ScanError) -> Self {
        Self::YamlParse(err)
    }
}

#[cfg(feature = "config-file")]
impl Error for ConfigLoadError {}

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

    /// Tries to load configuration from configuration file.
    #[cfg(feature = "config-file")]
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigLoadError> {
        let file_data = fs::read(path).await?;
        let file_text = String::from_utf8(file_data)?;
        let file_yaml = YamlLoader::load_from_str(&file_text)?;

        let mut config = Config::default();

        let yaml = match file_yaml.get(0) {
            Some(yaml) => yaml,
            None => return Ok(config),
        };

        let button_env_variable = &yaml["button_env_variable"];
        if !button_env_variable.is_badvalue() {
            let button_env_variable = button_env_variable
                .as_str()
                .ok_or_else(|| must_be_a_valid_yaml("button_env_variable", "string"))?;
            config.button_env_variable = String::from(button_env_variable);
        }

        let ipc = &yaml["ipc"];
        if !ipc.is_badvalue() {
            let server_type = &ipc["type"];
            if !server_type.is_badvalue() {
                #[allow(unused_variables)]
                let server_type = server_type
                    .as_str()
                    .ok_or_else(|| must_be_a_valid_yaml("type", "string"))?;
                #[cfg(not(feature = "ipc"))]
                {
                    eprintln!("Warning: asyncdwmblocks was compiled without `ipc` feature. Ignoring this option");
                }
                #[cfg(feature = "ipc")]
                {
                    match server_type {
                        "tcp" => {
                            #[cfg(not(feature = "tcp"))]
                            {
                                return Err(ConfigLoadError::ConfigError(
                                    "asyncdwmblocks was compiled without `tcp` feature".to_string(),
                                ));
                            }
                            #[cfg(feature = "tcp")]
                            {
                                config.server_type = ServerType::Tcp;
                            }
                        }
                        server_type => {
                            eprintln!(
                                "Warning: unrecognised option `{}`, using default `{}`",
                                server_type, config.server_type
                            );
                        }
                    }
                }
            }

            let tcp = &ipc["tcp"];
            if !tcp.is_badvalue() {
                let port = &tcp["port"];
                if !port.is_badvalue() {
                    #[allow(unused_variables)]
                    let port = port
                        .as_i64()
                        .ok_or_else(|| must_be_a_valid_yaml("port", "integer"))?;
                    #[cfg(not(feature = "tcp"))]
                    {
                        eprintln!("Warning: asyncdwmblocks was compiled without `tcp` feature. Ignoring this option");
                    }
                    #[cfg(feature = "tcp")]
                    {
                        if (1025..65535).contains(&port) {
                            config.tcp_port = port as u16;
                        } else {
                            return Err(ConfigLoadError::ConfigError(
                                "tcp port must be a value beetween 1025 and 65535".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            button_env_variable: String::from("BUTTON"),
            #[cfg(feature = "tcp")]
            tcp_port: 44000,
            #[cfg(feature = "ipc")]
            server_type: ServerType::Tcp,
        }
    }
}

#[cfg(feature = "config-file")]
fn must_be_a_valid_yaml(name: &str, yaml_type: &str) -> ConfigLoadError {
    ConfigLoadError::Syntax(format!("`{}` must be a valid yaml {}", name, yaml_type))
}
