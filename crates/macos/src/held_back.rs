//! Tells a window the program has open but macOS keeps off the screen.
//!
//! A panel that hides when its program is not in front, like a program's own drop-down list, is
//! left off the screen. The public window list shows it just like the many panels never opened;
//! only a private tag of the window server tells them apart.

use std::{
    ffi::{c_char, c_int, c_void, CStr},
    mem::transmute,
    sync::OnceLock,
};

const SKYLIGHT: &CStr = c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight";
const RTLD_LAZY: c_int = 1;
/// Measured on macOS 26: set from the moment a panel is shown until it is closed.
const ORDERED_IN: u64 = 0x2000;
const TAG_BITS: c_int = 64;

type MainConnection = unsafe extern "C" fn() -> c_int;
type GetWindowTags = unsafe extern "C" fn(c_int, u32, *mut u64, c_int) -> c_int;

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(library: *mut c_void, name: *const c_char) -> *mut c_void;
}

struct WindowServer {
    connection: c_int,
    get_tags: GetWindowTags,
}

/// Says whether the window is open and only macOS keeps it away. Without the private calls, it says no.
pub(crate) fn held_back(id: u32, on_screen: bool) -> bool {
    !on_screen && tags(id).is_some_and(is_ordered_in)
}

fn is_ordered_in(tags: u64) -> bool {
    tags & ORDERED_IN != 0
}

fn tags(id: u32) -> Option<u64> {
    let server = window_server()?;
    let mut tags = 0u64;
    // SAFETY: the pointer is a live u64, the 64 bits the call is told to write, and the connection
    // is this process's own.
    let error = unsafe { (server.get_tags)(server.connection, id, &raw mut tags, TAG_BITS) };
    (error == 0).then_some(tags)
}

fn window_server() -> Option<&'static WindowServer> {
    static LOADED: OnceLock<Option<WindowServer>> = OnceLock::new();
    LOADED.get_or_init(load).as_ref()
}

fn load() -> Option<WindowServer> {
    // SAFETY: the path is a NUL-terminated string naming a system framework.
    let library = unsafe { dlopen(SKYLIGHT.as_ptr(), RTLD_LAZY) };
    if library.is_null() {
        return None;
    }
    // SAFETY: the library was just opened and is never closed; the names are NUL-terminated.
    let (connection, get_tags) =
        unsafe { (dlsym(library, c"SLSMainConnectionID".as_ptr()), dlsym(library, c"SLSGetWindowTags".as_ptr())) };
    if connection.is_null() || get_tags.is_null() {
        return None;
    }
    // SAFETY: both pointers are non-null functions with these signatures, as called in the measured
    // macOS; the first takes no arguments and only returns this process's connection.
    unsafe {
        let connection = transmute::<*mut c_void, MainConnection>(connection)();
        let get_tags = transmute::<*mut c_void, GetWindowTags>(get_tags);
        Some(WindowServer { connection, get_tags })
    }
}

#[cfg(test)]
mod tests {
    use super::is_ordered_in;

    #[test]
    fn only_a_panel_shown_and_not_yet_closed_counts() {
        assert!(is_ordered_in(0x1000c6002), "open, kept off the screen");
        assert!(!is_ordered_in(0x1000c4002), "closed");
        assert!(!is_ordered_in(0x1000c0002), "never shown");
    }
}
