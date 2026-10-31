//! The events a program can be sent and the key combinations that produce them.

use std::str::FromStr;

use crate::{error::Error, geometry::Point};

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button {
    #[default]
    Left,
    Right,
    Middle,
}

impl FromStr for Button {
    type Err = Error;

    /// Reads `left`, `right` or `middle`.
    fn from_str(s: &str) -> Result<Self, Error> {
        match s {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "middle" => Ok(Self::Middle),
            _ => Err(Error::Invalid { what: "button", value: s.to_string() }),
        }
    }
}

/// A key, independent of the system's own numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable key, lower case, as on a US keyboard.
    Char(char),
    Return,
    Tab,
    Space,
    Delete,
    ForwardDelete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    /// Function key, 1 to 12.
    F(u8),
}

impl Key {
    fn from_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        let named = match name.as_str() {
            "return" | "enter" => Self::Return,
            "tab" => Self::Tab,
            "space" => Self::Space,
            "delete" | "backspace" => Self::Delete,
            "forward-delete" => Self::ForwardDelete,
            "esc" | "escape" => Self::Escape,
            "left" => Self::Left,
            "right" => Self::Right,
            "up" => Self::Up,
            "down" => Self::Down,
            "home" => Self::Home,
            "end" => Self::End,
            "pageup" => Self::PageUp,
            "pagedown" => Self::PageDown,
            _ => return Self::function_or_char(&name),
        };
        Some(named)
    }

    fn function_or_char(name: &str) -> Option<Self> {
        let mut chars = name.chars();
        let first = chars.next()?;
        if chars.next().is_none() {
            return Some(Self::Char(first));
        }
        let number: u8 = name.strip_prefix('f')?.parse().ok()?;
        (1..=12).contains(&number).then_some(Self::F(number))
    }
}

/// Modifier keys held during a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub option: bool,
    pub command: bool,
}

/// One key plus its modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl FromStr for Chord {
    type Err = Error;

    /// Reads forms like `return`, `a`, `f5` or `cmd+shift+a`.
    fn from_str(s: &str) -> Result<Self, Error> {
        let invalid = || Error::Invalid { what: "key", value: s.to_string() };
        let mut parts: Vec<&str> = s.split('+').collect();
        let key = parts.pop().and_then(Key::from_name).ok_or_else(invalid)?;
        let mut modifiers = Modifiers::default();
        for part in parts {
            match part.to_ascii_lowercase().as_str() {
                "shift" => modifiers.shift = true,
                "ctrl" | "control" => modifiers.control = true,
                "alt" | "opt" | "option" => modifiers.option = true,
                "cmd" | "command" => modifiers.command = true,
                _ => return Err(invalid()),
            }
        }
        Ok(Self { key, modifiers })
    }
}

/// One event for a program. Mouse positions are window points.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    MouseMove {
        at: Point,
    },
    MouseDown {
        at: Point,
        button: Button,
        clicks: u8,
    },
    MouseUp {
        at: Point,
        button: Button,
        clicks: u8,
    },
    MouseDrag {
        at: Point,
        button: Button,
    },
    /// `dy > 0` scrolls toward the end of the content, `dx > 0` toward its right, in lines.
    Scroll {
        at: Point,
        dx: i32,
        dy: i32,
    },
    Key {
        chord: Chord,
        down: bool,
    },
    /// One character typed as text, whatever the keyboard layout. `down` as in a key press.
    Text {
        ch: char,
        down: bool,
    },
}
