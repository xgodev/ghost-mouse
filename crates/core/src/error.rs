//! Every way a ghost-mouse operation can fail.

use std::path::PathBuf;

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
    /// The system leaves what it opens stopped at launch.
    #[error("macOS keeps {program} stopped at launch: its count of opened programs ran past 32 bits; restart the Mac")]
    HeldStopped { program: PathBuf },
    /// The system would accept the event and never deliver it.
    #[error("macOS does not deliver events to process {pid}: its count of opened programs ran past 32 bits; restart the Mac")]
    EventsLost { pid: i32 },
    /// The process shows no file panel.
    #[error("process {pid} shows no file panel")]
    NoFilePanel { pid: i32 },
    /// The panel never lists the path, so nothing was confirmed.
    #[error("the file panel does not list {path}")]
    NotInPanel { path: PathBuf },
    /// The panel only lets a list of rows be walked, and shows them some other way.
    #[error("the file panel is not in list view")]
    PanelNotInList,
    /// The panel's confirm button stays disabled for what was selected.
    #[error("the file panel does not accept what was selected")]
    PanelRefused,
    /// The panel stayed open after confirming.
    #[error("the file panel stayed open after confirming {path}")]
    PanelStayedOpen { path: PathBuf },
    /// The panel stayed open after cancelling.
    #[error("the file panel of process {pid} stayed open after cancelling")]
    PanelNotCancelled { pid: i32 },
    /// This program may not read other programs' windows.
    #[error("this program needs the Accessibility permission (System Settings > Privacy & Security)")]
    NoAccessibility,
    /// This operating system has no backend yet.
    #[error("this operating system is not supported yet")]
    Unsupported,
    /// A file or subprocess operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
