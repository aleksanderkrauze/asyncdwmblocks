# TODO list:

 - [ ] Use Default::default when creating Config by hand
 - [ ] Use consistent naming (does Block has a name of an id?)
 - [ ] Add option to use "Linux Abstract Socket Namespace" when target is Linux.
 - [ ] Add D-BUS IPC option
 - [ ] Block sends result though channel
 - [ ] Create patch for dwm (for clickable blocks)
 - [ ] Add README with instructions on how to build and install this package
 - [ ] Set metadata in Cargo.toml
 - [ ] Upload crate to crates.io
 - [ ] Write documentation for asyncdwmblocks library (lib.rs file)
 - [ ] Add option to daemonize asyncdwmblocks (bin)
 - [ ] In config file parse ~ as $HOME and possibly parse env variables
 - [ ] Look for `config.yml` when `config.yaml` is not found

# Idea

 - [ ] Add tooltip on hoover (?)
 - [ ] Add InternalConfig to Config and use it (server protocol?)
 - [ ] Put timeout at running Block's command (?)
 - [ ] Make warnings handling optional, maybe use some logging tool (?)
 - [ ] Don't end running server, if accepting connection failed (?)

# Done

 - [x] Add option to forcefully remove UDS file when starting UDS server (--force-remove-uds-file)
 - [x] Unify behaviour of Servers (their main loop)
 - [x] Filter out `\u{0}` chars from Block's output
 - [x] Use *norun* in tests
 - [x] Use `Self::x` in `match self { ... }`
 - [x] Solve problem of leaving Unix domain socket file opened when process is terminated by a signal!
 - [x] use `macro_rules!` to define generic server and notifier tests in `opaque.rs`
 - [x] Add better error messages when Unix domain socket file wasn't unlinked
 - [x] Add Unix domain sockets IPC option
 - [x] Write tests in `ipc::opaque`
 - [x] Write tests to check failing connections in servers and notifiers
 - [x] Remove Unix domain socket file after server finishes
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
