//! Plans what a program should receive and hands it to a system [`Backend`]; holds no system code.

mod backend;
mod error;
mod geometry;
mod ghost;
mod input;
pub mod plan;
mod png;

pub use backend::{Backend, Launch};
pub use error::{Error, Result};
pub use geometry::{Point, Window};
pub use ghost::{Ghost, Shot};
pub use input::{Button, Chord, Input, Key, Modifiers};
pub use png::png_size;
