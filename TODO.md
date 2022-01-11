# TODO list:

 - [ ] Move Block's id to StatusBar, StatusBar::new doesn't return error,
 loading StatusBar from config, StatusBar::default loads from Config::default
 - [ ] Block sends result though channel

 - [ ] Remove unused features from futures dependency
 - [ ] Write binaries
 - [ ] Create patch for dwm
 - [ ] Add README with instructions on how to build and install this package
 - [ ] Set metadata in Cargo.toml
 - [ ] Add InternalConfig to Config and use it (server protocol?)
 - [ ] Add BlocksConfig for default Blocks
 - [ ] Write documentation for asyncdwmblocks library (lib.rs file)
 - [ ] Put timeout at running Block's command
 - [ ] Add option to daemonise asyncdwmblocks (bin)
 - [x] Add LICENSE
 - [x] Load Configuration from config file
 - [x] Put TcpServer and TcpNotifier behind feature flag (tcp) and create feature management
