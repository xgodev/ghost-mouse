//! Lists the normal windows the system knows about.

use std::ffi::c_void;

use ghost_mouse_core::{Error, Result, Window};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFNumberType, CFString, CGRect};
use objc2_core_graphics::{
    kCGWindowBounds, kCGWindowIsOnscreen, kCGWindowLayer, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID, CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo, CGWindowListOption,
};

/// Returns layer-0 windows, of one process or of all of them.
pub(crate) fn list(pid: Option<i32>) -> Result<Vec<Window>> {
    let all = CGWindowListCopyWindowInfo(CGWindowListOption::OptionAll, 0)
        .ok_or_else(|| Error::Event("the window server gave no window list".to_string()))?;
    let mut windows = Vec::new();
    for index in 0..all.count() {
        // SAFETY: the index is in range and every element of this list is a dictionary, per the
        // CGWindowListCopyWindowInfo contract; `all` outlives the borrow.
        let info = unsafe { &*all.value_at_index(index).cast::<CFDictionary>() };
        if let Some(window) = read_window(info).filter(|w| pid.is_none_or(|p| p == w.pid)) {
            windows.push(window);
        }
    }
    Ok(windows)
}

fn read_window(info: &CFDictionary) -> Option<Window> {
    // SAFETY: these are constant strings exported by CoreGraphics, valid for the whole process.
    let (number, layer, owner_pid, owner, name, on_screen, bounds) = unsafe {
        (
            kCGWindowNumber,
            kCGWindowLayer,
            kCGWindowOwnerPID,
            kCGWindowOwnerName,
            kCGWindowName,
            kCGWindowIsOnscreen,
            kCGWindowBounds,
        )
    };
    if read_number(info, layer)? != 0 {
        return None;
    }
    let rect = read_rect(info, bounds)?;
    Some(Window {
        id: u32::try_from(read_number(info, number)?).ok()?,
        pid: i32::try_from(read_number(info, owner_pid)?).ok()?,
        owner: read_string(info, owner).unwrap_or_default(),
        title: read_string(info, name).unwrap_or_default(),
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
        on_screen: read_bool(info, on_screen),
    })
}

fn value(info: &CFDictionary, key: &CFString) -> *const c_void {
    // SAFETY: the dictionary is a live CFDictionary keyed by CFString, and `key` is a live CFString.
    unsafe { info.value((key as *const CFString).cast()) }
}

fn read_number(info: &CFDictionary, key: &CFString) -> Option<i64> {
    let number = value(info, key).cast::<CFNumber>();
    if number.is_null() {
        return None;
    }
    let mut out = 0i64;
    // SAFETY: this key always holds a CFNumber, kept alive by `info`, and `out` has the size of SInt64.
    let ok = unsafe { (*number).value(CFNumberType::SInt64Type, (&raw mut out).cast()) };
    ok.then_some(out)
}

fn read_string(info: &CFDictionary, key: &CFString) -> Option<String> {
    let text = value(info, key).cast::<CFString>();
    // SAFETY: this key holds a CFString when present, kept alive by `info`.
    (!text.is_null()).then(|| unsafe { (*text).to_string() })
}

fn read_bool(info: &CFDictionary, key: &CFString) -> bool {
    let flag = value(info, key).cast::<CFBoolean>();
    // SAFETY: this key holds a CFBoolean when present, kept alive by `info`.
    !flag.is_null() && unsafe { (*flag).value() }
}

fn read_rect(info: &CFDictionary, key: &CFString) -> Option<CGRect> {
    let bounds = value(info, key).cast::<CFDictionary>();
    if bounds.is_null() {
        return None;
    }
    let mut rect = CGRect::default();
    // SAFETY: this key holds a rectangle dictionary kept alive by `info`, and `rect` is writable.
    let ok = unsafe { CGRectMakeWithDictionaryRepresentation(Some(&*bounds), &raw mut rect) };
    ok.then_some(rect)
}
