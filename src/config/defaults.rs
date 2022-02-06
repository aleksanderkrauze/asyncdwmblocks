//! This module implements [Default] trait for config types.

use super::*;

fn default_statusbar_blocks() -> Vec<ConfigStatusBarBlock> {
    // # Example:
    //
    // vec![
    //   ConfigStatusBarBlock {
    //     name: "battery".to_string(),
    //     command: "battery.sh".to_string(),
    //     args: vec![],
    //     interval: Some(60),
    //   },
    //   ConfigStatusBarBlock {
    //     name: "backlight".to_string(),
    //     command: "backlight.sh".to_string(),
    //     args: vec![],
    //     interval: None,
    //    },
    // ]
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

#[cfg(feature = "uds")]
impl Default for ConfigIpcUnixDomainSocket {
    fn default() -> Self {
        Self {
            addr: PathBuf::from("/tmp/asyncdwmblocks.socket"),
            force_remove_uds_file: false,
            #[cfg(target_os = "linux")]
            abstract_namespace: false,
        }
    }
}

#[cfg(feature = "ipc")]
impl Default for ConfigIpc {
    fn default() -> Self {
        #[allow(unused_variables)]
        let server_type = {
            #[cfg(feature = "uds")]
            let server_type = ServerType::UnixDomainSocket;

            #[cfg(feature = "tcp")]
            let server_type = ServerType::Tcp;

            server_type
        };

        Self {
            server_type,
            #[cfg(feature = "tcp")]
            tcp: Default::default(),
            #[cfg(feature = "uds")]
            uds: Default::default(),
        }
    }
}

// Default is implemented by hand to improve readability.
#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            statusbar: Default::default(),
            block: Default::default(),
            #[cfg(feature = "ipc")]
            ipc: Default::default(),
        }
    }
}
