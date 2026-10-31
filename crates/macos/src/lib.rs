//! macOS backend: events go straight to one process, so the real cursor and front app stay put.

#![cfg(target_os = "macos")]

mod bundle;
mod events;
mod front;
mod location;
mod virtual_keys;
mod windows;

use std::{path::Path, process::Command, thread, time::Duration};

pub use events::cursor_position;
use ghost_mouse_core::{Backend, Error, Input, Launch, Result, Window};

/// The system backend for macOS.
#[derive(Debug, Default, Clone, Copy)]
pub struct MacBackend;

impl Backend for MacBackend {
    fn windows(&self, pid: Option<i32>) -> Result<Vec<Window>> {
        windows::list(pid)
    }

    fn send(&self, window: &Window, input: &Input) -> Result<()> {
        events::post(window, input)
    }

    fn wait(&self, time: Duration) {
        thread::sleep(time);
    }

    fn capture(&self, window: &Window, out: &Path) -> Result<()> {
        let output = Command::new("/usr/sbin/screencapture")
            .args(["-x", "-o", &format!("-l{}", window.id)])
            .arg(out)
            .output()?;
        if output.status.success() && out.exists() {
            return Ok(());
        }
        Err(Error::Capture(String::from_utf8_lossy(&output.stderr).trim().to_string()))
    }

    fn launch(&self, launch: &Launch) -> Result<i32> {
        bundle::launch(launch)
    }

    fn front_pid(&self) -> Option<i32> {
        front::front_pid()
    }

    fn activate(&self, pid: i32) -> Result<()> {
        front::activate(pid)
    }
}
