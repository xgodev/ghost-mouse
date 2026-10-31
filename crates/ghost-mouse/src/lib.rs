//! Drives any desktop program from outside, without touching the real mouse or the front app.

#[cfg(not(target_os = "macos"))]
mod unsupported;

pub use ghost_mouse_core::*;
#[cfg(target_os = "macos")]
pub use ghost_mouse_macos::cursor_position;

/// The backend of this operating system.
#[cfg(target_os = "macos")]
pub type SystemBackend = ghost_mouse_macos::MacBackend;
/// The backend of this operating system.
#[cfg(not(target_os = "macos"))]
pub type SystemBackend = unsupported::Unsupported;

/// Returns a [`Ghost`] for this operating system.
pub fn system() -> Ghost<SystemBackend> {
    Ghost::new(SystemBackend::default())
}
