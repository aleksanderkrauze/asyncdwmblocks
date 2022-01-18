//! This module defines Unix domain socket versions of [Server] and [Notifier].
//!
//! For more informations read documentations of [`UdsServer`] and [`UdsNotifier`].

pub mod notifier;
pub mod server;

pub use notifier::UdsNotifier;
pub use server::UdsServer;

use super::frame;
use super::{Notifier, Server};
