//! Reads and presses the native file panel through Accessibility, which reaches it behind other apps.
//!
//! Elements are found by identifier and rows by the file they stand for, so no language is involved.
//! Keys cannot be used: the panel lives in a system service and ignores events sent to a process.

use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::NonNull,
    thread,
    time::{Duration, Instant},
};

use ghost_mouse_core::{Error, Panel, PanelKind, Result};
use objc2_application_services::{AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::{CFArray, CFBoolean, CFRetained, CFString, CFType, CFURL};

const PATH_BYTES: usize = 4096;
const UI_POLL: Duration = Duration::from_millis(50);
/// How long a menu gets to open, and the confirm button to wake up after a selection.
const UI_ANSWER: Duration = Duration::from_secs(1);

const OPEN_PANEL: &str = "open-panel";
const SAVE_PANEL: &str = "save-panel";
const FILE_LIST: &str = "ListView";
const FOLDER_POPUP: &str = "where popup";
const NAME_FIELD: &str = "saveAsNameTextField";
const CONFIRM_BUTTON: &str = "OKButton";
const CANCEL_BUTTON: &str = "CancelButton";

/// One element of another program's interface.
struct Element(CFRetained<AXUIElement>);

impl Element {
    fn attribute(&self, name: &str) -> Option<CFRetained<CFType>> {
        let mut value: *const CFType = std::ptr::null();
        // SAFETY: the out pointer is valid; on success it holds a value this call owns.
        unsafe {
            let error = self.0.copy_attribute_value(&CFString::from_str(name), NonNull::from(&mut value));
            if error.0 != 0 {
                return None;
            }
            NonNull::new(value.cast_mut()).map(|value| CFRetained::from_raw(value))
        }
    }

    fn elements(&self, name: &str) -> Vec<Element> {
        let Some(array) = self.attribute(name).and_then(|value| value.downcast::<CFArray>().ok()) else {
            return vec![];
        };
        // SAFETY: an array of Core Foundation objects is what Accessibility hands out.
        let array = unsafe { array.cast_unchecked::<CFArray<CFType>>() };
        array.iter().filter_map(|item| item.downcast::<AXUIElement>().ok()).map(Element).collect()
    }

    fn text(&self, name: &str) -> Option<String> {
        Some(self.attribute(name)?.downcast::<CFString>().ok()?.to_string())
    }

    fn flag(&self, name: &str) -> bool {
        let value = self.attribute(name).and_then(|value| value.downcast::<CFBoolean>().ok());
        value.is_some_and(|value| value.as_bool())
    }

    fn set(&self, name: &str, value: &CFType) -> Result<()> {
        // SAFETY: both values are live Core Foundation objects.
        let error = unsafe { self.0.set_attribute_value(&CFString::from_str(name), value) };
        match error.0 {
            0 => Ok(()),
            code => Err(Error::Event(format!("the file panel refused {name}: {code}"))),
        }
    }

    /// The panel may answer with an error for a press that worked, as it closes or changes folder:
    /// the outcome is what shows whether it landed.
    fn press(&self) {
        // SAFETY: the action name is a live string.
        unsafe { self.0.perform_action(&CFString::from_str("AXPress")) };
    }

    fn is(&self, role: &str) -> bool {
        self.text("AXRole").as_deref() == Some(role)
    }

    /// Lists are not searched: the file list alone can hold thousands of rows.
    fn find(&self, identifier: &str) -> Option<Element> {
        for child in self.elements("AXChildren") {
            if child.text("AXIdentifier").as_deref() == Some(identifier) {
                return Some(child);
            }
            if !child.is("AXOutline") {
                if let Some(found) = child.find(identifier) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// The file a row of the file list stands for; its file reference resolves to the real path.
    fn row_path(&self) -> Option<PathBuf> {
        let cell = self.elements("AXChildren").into_iter().next()?;
        let url = cell.elements("AXChildren").into_iter().find_map(|field| field.attribute("AXURL"))?;
        path_of(&*url.downcast::<CFURL>().ok()?).filter(|path| !path.as_os_str().is_empty())
    }
}

fn path_of(url: &CFURL) -> Option<PathBuf> {
    let mut bytes = vec![0u8; PATH_BYTES];
    // SAFETY: the buffer holds the length given.
    let done = unsafe { url.file_system_representation(true, bytes.as_mut_ptr(), PATH_BYTES as isize) };
    if !done {
        return None;
    }
    let end = bytes.iter().position(|byte| *byte == 0)?;
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(&bytes[..end])))
}

fn panel_kind(element: &Element) -> Option<PanelKind> {
    match element.text("AXIdentifier").as_deref() {
        Some(OPEN_PANEL) => Some(PanelKind::Open),
        Some(SAVE_PANEL) => Some(PanelKind::Save),
        _ => None,
    }
}

/// A panel is a window of the program, or a sheet on one of its windows.
fn find_panel(pid: i32) -> Result<Option<(Element, PanelKind)>> {
    // SAFETY: the system call takes no pointers.
    if !unsafe { AXIsProcessTrusted() } {
        return Err(Error::NoAccessibility);
    }
    // SAFETY: any process id is accepted; a wrong one yields an element with no attributes.
    let app = Element(unsafe { AXUIElement::new_application(pid) });
    for window in app.elements("AXWindows") {
        if let Some(kind) = panel_kind(&window) {
            return Ok(Some((window, kind)));
        }
        for sheet in window.elements("AXChildren").into_iter().filter(|child| child.is("AXSheet")) {
            if let Some(kind) = panel_kind(&sheet) {
                return Ok(Some((sheet, kind)));
            }
        }
    }
    Ok(None)
}

fn open_panel(pid: i32) -> Result<Element> {
    Ok(find_panel(pid)?.ok_or(Error::NoFilePanel { pid })?.0)
}

fn rows(panel: &Element) -> Result<Vec<(PathBuf, Element)>> {
    let list = panel.find(FILE_LIST).ok_or(Error::PanelNotInList)?;
    Ok(list.elements("AXRows").into_iter().filter_map(|row| Some((row.row_path()?, row))).collect())
}

fn row(pid: i32, wanted: &Path) -> Result<(Element, Element)> {
    let panel = open_panel(pid)?;
    let found = rows(&panel)?.into_iter().find(|(path, _)| path == wanted).map(|(_, row)| row);
    let row = found.ok_or_else(|| Error::NotInPanel { path: wanted.to_path_buf() })?;
    Ok((panel, row))
}

fn control(pid: i32, identifier: &str) -> Result<Element> {
    let found = open_panel(pid)?.find(identifier);
    found.ok_or_else(|| Error::Event(format!("the file panel has no {identifier}")))
}

pub(crate) fn read(pid: i32) -> Result<Option<Panel>> {
    let Some((panel, kind)) = find_panel(pid)? else {
        return Ok(None);
    };
    let rows = rows(&panel)?.into_iter().map(|(path, _)| path).collect();
    Ok(Some(Panel { kind, rows }))
}

/// Opens the folder menu, whose first group runs from the folder shown up to the computer.
pub(crate) fn up(pid: i32) -> Result<bool> {
    let popup = control(pid, FOLDER_POPUP)?;
    popup.press();
    let started = Instant::now();
    let menu = loop {
        if let Some(menu) = popup.elements("AXChildren").into_iter().find(|child| child.is("AXMenu")) {
            break menu;
        }
        if started.elapsed() >= UI_ANSWER {
            return Err(Error::Event("the file panel did not open its folder menu".to_string()));
        }
        thread::sleep(UI_POLL);
    };
    let items = menu.elements("AXChildren");
    let mut folders = items.iter().take_while(|item| item.text("AXTitle").is_some_and(|title| !title.is_empty()));
    let Some(above) = folders.nth(1) else {
        // SAFETY: the action name is a live string.
        unsafe { menu.0.perform_action(&CFString::from_str("AXCancel")) };
        return Ok(false);
    };
    above.press();
    Ok(true)
}

pub(crate) fn expand(pid: i32, wanted: &Path) -> Result<()> {
    row(pid, wanted)?.1.set("AXDisclosing", CFBoolean::new(true))
}

pub(crate) fn select(pid: i32, wanted: &Path) -> Result<()> {
    let (panel, row) = row(pid, wanted)?;
    let list = panel.find(FILE_LIST).ok_or(Error::PanelNotInList)?;
    list.set("AXSelectedRows", &CFArray::from_objects(&[&*row.0]))
}

pub(crate) fn name(pid: i32, name: &str) -> Result<()> {
    control(pid, NAME_FIELD)?.set("AXValue", &CFString::from_str(name))
}

pub(crate) fn confirm(pid: i32) -> Result<()> {
    let button = control(pid, CONFIRM_BUTTON)?;
    let started = Instant::now();
    while !button.flag("AXEnabled") {
        if started.elapsed() >= UI_ANSWER {
            return Err(Error::PanelRefused);
        }
        thread::sleep(UI_POLL);
    }
    button.press();
    Ok(())
}

pub(crate) fn cancel(pid: i32) -> Result<()> {
    control(pid, CANCEL_BUTTON)?.press();
    Ok(())
}
