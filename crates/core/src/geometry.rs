//! Points inside a window and the description of a window.

use std::{fmt, str::FromStr};

use serde::Serialize;

use crate::error::Error;

/// A position in window points, counted from the window's top-left corner, title bar included.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

impl FromStr for Point {
    type Err = Error;

    /// Reads `x,y`.
    fn from_str(s: &str) -> Result<Self, Error> {
        let invalid = || Error::Invalid { what: "point", value: s.to_string() };
        let (x, y) = s.split_once(',').ok_or_else(invalid)?;
        let number = |v: &str| v.trim().parse::<f64>().ok().filter(|n| n.is_finite()).ok_or_else(invalid);
        Ok(Self { x: number(x)?, y: number(y)? })
    }
}

/// One window of a running program, with its position on the desktop in points.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Window {
    pub id: u32,
    pub pid: i32,
    pub owner: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub on_screen: bool,
    /// 0 for a normal window; anything else floats over them, like a program's open list.
    pub layer: i32,
    /// Open, but kept off the screen by the system while the program is not in front. It takes
    /// clicks and pictures all the same.
    pub held_back: bool,
}

/// A rectangle in window points, counted like [`Point`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// The point halfway across and halfway down.
    pub fn center(&self) -> Point {
        Point { x: self.x + self.width / 2.0, y: self.y + self.height / 2.0 }
    }
}
