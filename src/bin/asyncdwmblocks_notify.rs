use std::error::Error;
use std::fmt;
use std::process;

use clap::{App, Arg};
use tokio::runtime;

use asyncdwmblocks::{
    block::BlockRunMode,
    config::Config,
    ipc::{Notifier, OpaqueNotifier},
    statusbar::BlockRefreshMessage,
};

#[derive(Debug, PartialEq, Clone)]
struct CliArgsParseError(String);

impl fmt::Display for CliArgsParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for CliArgsParseError {}

fn parse_cli_args() -> Result<BlockRefreshMessage, CliArgsParseError> {
    let app = App::new("asyncdwmblocks-notifier")
        .about("Send notifications to asyncdwmblocks")
        .arg(
            Arg::new("block")
                .required(true)
                .help("Name of a block that you wish to reload"),
        )
        .arg(
            Arg::new("button")
                .short('b')
                .long("button")
                .takes_value(true)
                .help("Reload given block as clicked with provided <button>"),
        );

    let matches = app.get_matches();
    let block = matches
        .value_of("block")
        .ok_or_else(|| CliArgsParseError(String::from("Specify which block should be reloaded")))?;
    let button: Option<u8> = match matches.value_of("button").map(str::parse::<u8>) {
        Some(Ok(v)) => Some(v),
        None => None,
        Some(Err(e)) => {
            return Err(CliArgsParseError(format!(
                "Button's option must be a number: {}",
                e
            )))
        }
    };

    Ok(BlockRefreshMessage {
        name: block.to_string(),
        mode: match button {
            Some(b) => BlockRunMode::Button(b),
            None => BlockRunMode::Normal,
        },
    })
}

async fn run() -> Result<(), Box<dyn Error>> {
    let msg = parse_cli_args()?;
    let config = Config::get_config().await?.arc();

    let mut notifier = OpaqueNotifier::new(config);

    notifier.push_message(msg);
    notifier.send_messages().await?;

    Ok(())
}

fn main() {
    let rt = runtime::Runtime::new().expect("Failed to create tokio runtime.");

    let result = rt.block_on(run());
    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
