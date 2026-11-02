//! macOS backend: events go straight to one process, so the real cursor and front app stay put.

#![cfg(target_os = "macos")]

mod bundle;
mod controls;
mod element;
mod events;
mod front;
mod held_back;
mod location;
mod panel;
mod virtual_keys;
mod web;
mod windows;

use std::{path::Path, process::Command, thread, time::Duration};

pub use events::cursor_position;
use ghost_mouse_core::{Backend, Control, Error, Input, Launch, Panel, Point, Result, Window};

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

    fn press_at(&self, window: &Window, at: Point) -> bool {
        web::press(window, at)
    }

    fn check_delivery(&self, pid: i32) -> Result<()> {
        front::check_delivery(pid)
    }

    fn controls(&self, window: &Window) -> Result<Vec<Control>> {
        controls::read(window)
    }

    fn panel(&self, pid: i32) -> Result<Option<Panel>> {
        panel::read(pid)
    }

    fn panel_up(&self, pid: i32) -> Result<bool> {
        panel::up(pid)
    }

    fn panel_expand(&self, pid: i32, row: &Path) -> Result<()> {
        panel::expand(pid, row)
    }

    fn panel_select(&self, pid: i32, row: &Path) -> Result<()> {
        panel::select(pid, row)
    }

    fn panel_name(&self, pid: i32, name: &str) -> Result<()> {
        panel::name(pid, name)
    }

    fn panel_confirm(&self, pid: i32) -> Result<()> {
        panel::confirm(pid)
    }

    fn panel_cancel(&self, pid: i32) -> Result<()> {
        panel::cancel(pid)
    }
}
