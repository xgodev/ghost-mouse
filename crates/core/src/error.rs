//! Every way a ghost-mouse operation can fail.

use thiserror::Error;

use crate::geometry::Point;

/// Shorthand for results of this library.
pub type Result<T> = std::result::Result<T, Error>;

/// Why an operation could not be done.
#[derive(Debug, Error)]
pub enum Error {
    /// The process has no window that can receive events.
    #[error("process {pid} has no visible window")]
    NoWindow { pid: i32 },
    /// The requested window id does not belong to the process.
    #[error("process {pid} has no window {window}")]
    UnknownWindow { pid: i32, window: u32 },
    /// A point falls outside the window, so the event would hit nothing.
    #[error("point {at} is outside window {window} ({width}x{height})")]
    OutsideWindow { at: Point, window: u32, width: f64, height: f64 },
    /// A command-line or API value could not be understood.
    #[error("invalid {what}: {value}")]
    Invalid { what: &'static str, value: String },
    /// The system refused to build the event.
    #[error("could not build the event: {0}")]
    Event(String),
    /// The picture could not be taken or read.
    #[error("could not take the picture: {0}")]
    Capture(String),
    /// The program could not be opened in the background.
    #[error("could not open the program: {0}")]
    Launch(String),
    /// The opened program took the front and it could not be given back.
    #[error("process {pid} took the front and it could not be given back to process {front}")]
    FrontLost { pid: i32, front: i32 },
    /// This operating system has no backend yet.
    #[error("this operating system is not supported yet")]
    Unsupported,
    /// A file or subprocess operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
