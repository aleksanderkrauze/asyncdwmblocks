//! This module contains a collection of functions
//! and structs that are used through this library.
//!
//! These "things" do one thing and do it well.
//! For more information consult their documentation.
//! You may start looking at pub exports.

pub mod pipe;
pub mod split;

pub use pipe::mpsc_pipe;
pub use pipe::mpsc_pipe_translate;
pub use split::SplitAtRN;
