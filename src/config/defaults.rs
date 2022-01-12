//! This module implements [Default] trait for config types.

use super::*;

fn default_statusbar_blocks() -> Vec<ConfigStatusBarBlock> {
    vec![]
}

impl Default for ConfigStatusBar {
    fn default() -> Self {
        Self {
            delimiter: String::from(" "),
            blocks: default_statusbar_blocks(),
        }
    }
}

impl Default for ConfigBlock {
    fn default() -> Self {
        Self {
            clicked_env_variable: String::from("BUTTON"),
        }
    }
}

#[cfg(feature = "tcp")]
impl Default for ConfigIpcTcp {
    fn default() -> Self {
        Self { port: 44000 }
    }
}

#[cfg(feature = "ipc")]
impl Default for ConfigIpc {
    fn default() -> Self {
        let server_type = {
            #[cfg(feature = "tcp")]
            let server_type = ServerType::Tcp;

            server_type
        };

        Self {
            server_type,
            #[cfg(feature = "tcp")]
            tcp: ConfigIpcTcp::default(),
        }
    }
}

// Default is implemented by hand to improve readability.
#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            statusbar: ConfigStatusBar::default(),
            block: ConfigBlock::default(),
            #[cfg(feature = "ipc")]
            ipc: ConfigIpc::default(),
        }
    }
}
