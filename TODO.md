# TODO list:

 - [ ] Block sends result though channel
 - [ ] Make warnings handling optional
 - [ ] Restructure Config and use serde to load it from file
 - [ ] Write binaries
 - [ ] Create patch for dwm
 - [ ] Add README with instructions on how to build and install this package
 - [ ] Set metadata in Cargo.toml
 - [ ] Add InternalConfig to Config and use it (server protocol?)
 - [ ] Write documentation for asyncdwmblocks library (lib.rs file)
 - [ ] Add option to daemonise asyncdwmblocks (bin)
 - [ ] Put timeout at running Block's command (?)
 - [ ] Remove unused features from futures dependency (?)
 - [x] Add BlocksConfig for default Blocks
 - [x] Add LICENSE
 - [x] Load Configuration from config file
 - [x] Put TcpServer and TcpNotifier behind feature flag (tcp) and create feature management
 - [x] Move Block's id to StatusBar, StatusBar::new doesn't return error,
 loading StatusBar from config, StatusBar::default loads from Config::default
