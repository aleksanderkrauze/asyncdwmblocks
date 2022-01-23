#![allow(unused_imports)]

use std::error::Error;
use std::process;
use std::sync::Arc;

use tokio::runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{broadcast, mpsc, oneshot};

#[cfg(feature = "ipc")]
use asyncdwmblocks::ipc::{OpaqueServer, Server};
use asyncdwmblocks::{config::Config, statusbar::StatusBar, x11};

// Some channels are not used without some features
#[allow(unused_variables, unused_mut, non_snake_case)]
async fn run() -> Result<(), Box<dyn Error>> {
    let x11 = x11::X11Connection::new()?;

    let config = Config::get_config().await?.arc();
    let mut statusbar = StatusBar::from(Arc::clone(&config));

    // This channel is used to catch informations
    // about OS signals and send them to different
    // running tasks to enable them to perform a cleanup
    // before gracefully shutting down.
    let (termination_signal_sender, termination_signal_receiver) = broadcast::channel(8);
    let mut termination_signal_receiver_statusbar = termination_signal_sender.subscribe();
    let mut termination_signal_receiver_main = termination_signal_sender.subscribe();
    let termination_signal_sender_statusbar = termination_signal_sender.clone();

    // This channel is used to tell other tasks that
    // server ended with error.
    let (server_termination_error_sender, mut server_termination_error_receiver) =
        broadcast::channel(8);

    // This channel is used to pass servers error and return it from this function.
    #[cfg(feature = "ipc")]
    let (server_error_sender, mut server_error_receiver) = oneshot::channel();

    // This channel is used by IPC server to send BlockRefreshMessages.
    let (server_sender, server_receiver) = mpsc::channel(8);

    // This channel is used to send computed status bar from
    // statusbar task to update xroot name task.
    let (statusbar_sender, mut statusbar_receiver) = mpsc::channel(8);

    // OS signals
    let mut SIGHUP = signal(SignalKind::hangup())?;
    let mut SIGINT = signal(SignalKind::interrupt())?;
    let mut SIGQUIT = signal(SignalKind::quit())?;
    let mut SIGTERM = signal(SignalKind::terminate())?;
    tokio::spawn(async move {
        tokio::select! {
            _ = SIGHUP.recv() => {},
            _ = SIGINT.recv() => {},
            _ = SIGQUIT.recv() => {},
            _ = SIGTERM.recv() => {},
        };

        let _ = termination_signal_sender.send(());
    });

    // IPC server
    #[cfg(feature = "ipc")]
    tokio::spawn(async move {
        let mut server = OpaqueServer::new(
            server_sender,
            termination_signal_receiver,
            Arc::clone(&config),
        );

        if let Err(e) = server.run().await {
            // If sending failed that mean that we are already finishing
            let _ = server_error_sender.send(e);
            let _ = server_termination_error_sender.send(());
        }
    });

    // Statusbar
    tokio::spawn(async move {
        tokio::select! {
            _ = statusbar.run(statusbar_sender, server_receiver) => {
                let _ = termination_signal_sender_statusbar.send(());
            },

            Ok(()) = server_termination_error_receiver.recv() => {},
            _ = termination_signal_receiver_statusbar.recv() => {},
        };
    });

    // Updating xroot name
    tokio::spawn(async move {
        while let Some(msg) = statusbar_receiver.recv().await {
            x11.set_root_name(&msg);
        }
    });

    // Waiting for gracefully shutdown
    #[cfg(feature = "ipc")]
    {
        tokio::select! {
            _ = termination_signal_receiver_main.recv() => {}
            Ok(err) = &mut server_error_receiver => {
                return Err(Box::new(err))
            }
        }
    }
    #[cfg(not(feature = "ipc"))]
    {
        let _ = termination_signal_receiver_main.recv().await;
    }

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
