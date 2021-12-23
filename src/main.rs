mod blocks;
mod x11;

use std::error::Error;
use std::process;

use tokio::runtime;

async fn run() -> Result<(), Box<dyn Error>> {
    let x11 = x11::X11Connection::new()?;
    x11.set_root_name("test");

    Ok(())
}

fn main() {
    let rt = runtime::Builder::new_multi_thread()
        .build()
        .expect("Failed to create tokio runtime.");

    let result = rt.block_on(run());
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
