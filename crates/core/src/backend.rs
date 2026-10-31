//! What each operating system must provide.

use std::{path::Path, path::PathBuf, time::Duration};

use crate::{error::Result, geometry::Window, input::Input};

/// A program to open in the background.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Launch {
    /// A `.app` bundle or a bare executable.
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// System operations. Only `activate` may change the front app, and nothing may move the real cursor.
pub trait Backend {
    /// Lists normal windows, of one process or of all of them.
    fn windows(&self, pid: Option<i32>) -> Result<Vec<Window>>;
    /// Delivers one event straight to the window's process.
    fn send(&self, window: &Window, input: &Input) -> Result<()>;
    /// Blocks the caller for the given time.
    fn wait(&self, time: Duration);
    /// Writes a PNG of the window, even if other windows cover it.
    fn capture(&self, window: &Window, out: &Path) -> Result<()>;
    /// Opens a program without taking the focus and returns its process id.
    fn launch(&self, launch: &Launch) -> Result<i32>;
    /// The process of the app in front, when the system can tell.
    fn front_pid(&self) -> Option<i32>;
    /// Puts a process back in front.
    fn activate(&self, pid: i32) -> Result<()>;
}
