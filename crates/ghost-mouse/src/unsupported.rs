//! Stands in on systems with no backend yet, so everything still builds and fails clearly.

use std::{path::Path, time::Duration};

use ghost_mouse_core::{Backend, Error, Input, Launch, Result, Window};

/// Answers every operation with [`Error::Unsupported`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Unsupported;

impl Backend for Unsupported {
    fn windows(&self, _pid: Option<i32>) -> Result<Vec<Window>> {
        Err(Error::Unsupported)
    }
    fn send(&self, _window: &Window, _input: &Input) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn wait(&self, time: Duration) {
        std::thread::sleep(time);
    }
    fn capture(&self, _window: &Window, _out: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn launch(&self, _launch: &Launch) -> Result<i32> {
        Err(Error::Unsupported)
    }
    fn front_pid(&self) -> Option<i32> {
        None
    }
    fn activate(&self, _pid: i32) -> Result<()> {
        Err(Error::Unsupported)
    }
}
