# TODO list:

 - [ ] Add Unix domain sockets ipc option
 - [ ] Add D-BUS ipc option
 - [ ] Use `Self::x` in `match self { ... }`
 - [ ] Remove uds file after server finishes
 - [ ] Write tests to check failing connections in servers and notifiers
 - [ ] Write tests in `ipc::opaque`
 - [ ] Block sends result though channel
 - [ ] Create patch for dwm
 - [ ] Add README with instructions on how to build and install this package
 - [ ] Set metadata in Cargo.toml
 - [ ] Upload crate to crates.io
 - [ ] Write documentation for asyncdwmblocks library (lib.rs file)
 - [ ] Add option to daemonize asyncdwmblocks (bin)
 - [ ] Use *norun* in tests
 - [ ] What to do if in block's output is `'\0'`? It will cause `X11Connection::set_root_name` to panic!

# Idea

 - [ ] Add tooltip on hoover (?)
 - [ ] Add InternalConfig to Config and use it (server protocol?)
 - [ ] Put timeout at running Block's command (?)
 - [ ] Make warnings handling optional, maybe use some logging tool (?)

# Done

 - [x] Filter through io::Errors and give more specific description
 - [x] Write binaries
 - [x] Remove unused features from futures dependency
 - [x] Restructure Config and use serde to load it from file
 - [x] Add BlocksConfig for default Blocks
 - [x] Add LICENSE
 - [x] Load Configuration from config file
 - [x] Put TcpServer and TcpNotifier behind feature flag (tcp) and create feature management
 - [x] Move Block's id to StatusBar, StatusBar::new doesn't return error,
 loading StatusBar from config, StatusBar::default loads from Config::default
