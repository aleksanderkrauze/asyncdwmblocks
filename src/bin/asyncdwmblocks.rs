use std::error::Error;
use std::process;
use std::sync::Arc;

use tokio::runtime;
use tokio::sync::{mpsc, oneshot};

#[cfg(feature = "ipc")]
use asyncdwmblocks::ipc::{self, Server};
use asyncdwmblocks::{config::Config, statusbar::StatusBar, x11};

// Some channels are not used without some features
#[allow(unused_variables, unused_mut)]
async fn run() -> Result<(), Box<dyn Error>> {
    let x11 = x11::X11Connection::new()?;

    let config = Config::get_config().await?.arc();
    let mut statusbar = StatusBar::from(Arc::clone(&config));

    let (server_sender, server_receiver) = mpsc::channel(8);
    #[cfg(feature = "ipc")]
    let (server_error_sender, mut server_error_receiver) = oneshot::channel();
    let (termination_sender, mut termination_receiver) = oneshot::channel::<()>();
    #[cfg(feature = "ipc")]
    tokio::spawn(async move {
        let server = ipc::get_server(server_sender, Arc::clone(&config));
        if let Err(e) = server.run().await {
            // If sending failed that mean that we are already finishing
            let _ = server_error_sender.send(e);
            let _ = termination_sender.send(());
        }
    });

    let (statusbar_sender, mut statusbar_receiver) = mpsc::channel(8);
    tokio::spawn(async move {
        statusbar.run(statusbar_sender, server_receiver).await;
    });

    tokio::spawn(async move {
        while let Some(msg) = statusbar_receiver.recv().await {
            x11.set_root_name(&msg);
        }
    });

    #[cfg(feature = "ipc")]
    tokio::select! {
        Ok(error) = &mut server_error_receiver => Err(Box::new(error)),
        Ok(()) = &mut termination_receiver => Ok(())
    }

    #[cfg(not(feature = "ipc"))]
    {
        let _ = termination_receiver.await;
        Ok(())
    }
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
