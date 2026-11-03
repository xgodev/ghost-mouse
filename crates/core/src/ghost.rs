//! The public operations: picks the window, checks the points and runs each gesture.

use std::{path::Path, path::PathBuf, time::Duration};

use serde::Serialize;

use crate::{
    backend::{Backend, Launch},
    error::{Error, Result},
    geometry::{Point, Window},
    input::{Button, Chord},
    plan::{self, Step},
    png::png_size,
};

const WINDOW_POLL: Duration = Duration::from_millis(100);
/// Some programs take the front a moment after their window shows.
const FRONT_WATCH: Duration = Duration::from_secs(3);

/// A picture taken of a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shot {
    pub path: PathBuf,
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Picture pixels per window point.
    pub scale: f64,
}

impl Shot {
    /// Converts a position read on the picture into the window point to click.
    pub fn to_point(&self, pixel_x: f64, pixel_y: f64) -> Point {
        Point { x: pixel_x / self.scale, y: pixel_y / self.scale }
    }
}

/// Drives programs through one system backend.
pub struct Ghost<B> {
    backend: B,
}

impl<B: Backend> Ghost<B> {
    /// Wraps a backend.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Gives access to the backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Lists windows of one process, or of all when `pid` is `None`.
    pub fn windows(&self, pid: Option<i32>) -> Result<Vec<Window>> {
        self.backend.windows(pid)
    }

    /// Picks the window a person would call the program's window: visible, named, largest.
    pub fn main_window(&self, pid: i32) -> Result<Window> {
        let area = |w: &Window| w.width * w.height;
        self.backend
            .windows(Some(pid))?
            .into_iter()
            .filter(|w| w.layer == 0 && w.on_screen && area(w) > 0.0)
            .max_by(|a, b| (!a.title.is_empty()).cmp(&!b.title.is_empty()).then_with(|| area(a).total_cmp(&area(b))))
            .ok_or(Error::NoWindow { pid })
    }

    /// Finds one window of the process by id.
    pub fn window(&self, pid: i32, id: u32) -> Result<Window> {
        self.backend
            .windows(Some(pid))?
            .into_iter()
            .find(|w| w.id == id)
            .ok_or(Error::UnknownWindow { pid, window: id })
    }

    /// Waits until the process shows a window, up to `timeout`.
    pub fn wait_for_window(&self, pid: i32, timeout: Duration) -> Result<Window> {
        let mut waited = Duration::ZERO;
        loop {
            match self.main_window(pid) {
                Err(Error::NoWindow { .. }) if waited < timeout => {
                    self.backend.wait(WINDOW_POLL);
                    waited += WINDOW_POLL;
                }
                result => return result,
            }
        }
    }

    /// Opens a program in the background and returns its main window.
    ///
    /// A program may bring itself forward as it starts; the front goes back to where it was.
    pub fn open(&self, launch: &Launch, timeout: Duration) -> Result<Window> {
        let front = self.backend.front_pid();
        let pid = self.backend.launch(launch)?;
        let window = self.wait_for_window(pid, timeout);
        if let Some(front) = front.filter(|front| *front != pid) {
            self.keep_front(pid, front)?;
        }
        window
    }

    /// Only undoes the program itself: a person moving to another app is left alone.
    pub(crate) fn keep_front(&self, pid: i32, front: i32) -> Result<()> {
        let mut waited = Duration::ZERO;
        loop {
            if self.backend.front_pid() == Some(pid) {
                self.backend.activate(front).map_err(|_| Error::FrontLost { pid, front })?;
            }
            if waited >= FRONT_WATCH {
                return Ok(());
            }
            self.backend.wait(WINDOW_POLL);
            waited += WINDOW_POLL;
        }
    }

    /// Rests the pointer over a point, without moving the real cursor.
    pub fn hover(&self, window: &Window, at: Point) -> Result<()> {
        inside(window, at)?;
        self.run(window, plan::hover(at))
    }

    /// Clicks a point of the window.
    pub fn click(&self, window: &Window, at: Point, button: Button, clicks: u8) -> Result<()> {
        inside(window, at)?;
        if clicks == 0 {
            return Err(Error::Invalid { what: "click count", value: clicks.to_string() });
        }
        // Pressing goes by the system's own windows and would reach the one under a floating list.
        if window.layer == 0 && button == Button::Left && clicks == 1 && self.backend.press_at(window, at) {
            return Ok(());
        }
        self.run(window, plan::click(at, button, clicks))
    }

    /// Drags between two points of the window.
    pub fn drag(&self, window: &Window, from: Point, to: Point, button: Button, steps: u32) -> Result<()> {
        inside(window, from)?;
        inside(window, to)?;
        self.run(window, plan::drag(from, to, button, steps))
    }

    /// Scrolls at a point of the window.
    pub fn scroll(&self, window: &Window, at: Point, dx: i32, dy: i32) -> Result<()> {
        inside(window, at)?;
        if window.layer == 0 && self.backend.scroll_at(window, at, dx, dy) {
            return Ok(());
        }
        self.run(window, plan::scroll(at, dx, dy))
    }

    /// Presses one key combination.
    pub fn press(&self, window: &Window, chord: Chord) -> Result<()> {
        self.run(window, plan::press(chord))
    }

    /// Types text.
    pub fn type_text(&self, window: &Window, text: &str) -> Result<()> {
        self.run(window, plan::type_text(text))
    }

    /// Takes a picture of the window, even when it is covered.
    pub fn shot(&self, window: &Window, out: &Path) -> Result<Shot> {
        self.backend.capture(window, out)?;
        let bytes = std::fs::read(out)?;
        let (pixel_width, pixel_height) =
            png_size(&bytes).ok_or_else(|| Error::Capture(format!("{} is not a PNG", out.display())))?;
        if window.width <= 0.0 {
            return Err(Error::Capture(format!("window {} has no size", window.id)));
        }
        let scale = f64::from(pixel_width) / window.width;
        Ok(Shot { path: out.to_path_buf(), pixel_width, pixel_height, scale })
    }

    fn run(&self, window: &Window, steps: Vec<Step>) -> Result<()> {
        self.backend.check_delivery(window.pid)?;
        for step in steps {
            match step {
                Step::Send(input) => self.backend.send(window, &input)?,
                Step::Wait(time) => self.backend.wait(time),
            }
        }
        Ok(())
    }
}

fn inside(window: &Window, at: Point) -> Result<()> {
    if (0.0..window.width).contains(&at.x) && (0.0..window.height).contains(&at.y) {
        return Ok(());
    }
    Err(Error::OutsideWindow { at, window: window.id, width: window.width, height: window.height })
}
