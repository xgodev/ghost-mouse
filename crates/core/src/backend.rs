//! What each operating system must provide.

use std::{path::Path, path::PathBuf, time::Duration};

use crate::{
    error::{Error, Result},
    geometry::{Point, Window},
    input::Input,
    panel::Panel,
};

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

    /// Presses what sits at the point, for what events cannot reach behind other apps; `false` leaves
    /// the click to events.
    fn press_at(&self, _window: &Window, _at: Point) -> bool {
        false
    }

    /// The file panel the process shows, if any.
    fn panel(&self, _pid: i32) -> Result<Option<Panel>> {
        Err(Error::Unsupported)
    }
    /// Shows the folder above the one in the panel; `false` when there is none.
    fn panel_up(&self, _pid: i32) -> Result<bool> {
        Err(Error::Unsupported)
    }
    /// Lists a folder's rows under its own row.
    fn panel_expand(&self, _pid: i32, _row: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Selects one row.
    fn panel_select(&self, _pid: i32, _row: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Fills in the name of a file to save.
    fn panel_name(&self, _pid: i32, _name: &str) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Presses the button that accepts the panel.
    fn panel_confirm(&self, _pid: i32) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Presses the button that dismisses the panel.
    fn panel_cancel(&self, _pid: i32) -> Result<()> {
        Err(Error::Unsupported)
    }
}
