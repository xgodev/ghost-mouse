//! Reads a window's interface elements through Accessibility, which answers for windows behind other apps.

use std::{
    ffi::{c_char, c_int, c_void, CStr},
    mem::transmute,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use ghost_mouse_core::{Control, Error, Frame, Result, Window};
use objc2_application_services::{AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::{CFBoolean, CFNumber, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize};

use crate::element::{self, Element};

const APPLICATION_SERVICES: &CStr = c"/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices";
const RTLD_LAZY: c_int = 1;
/// More than this and a window is refused rather than read part way. Measured: 37 elements in Calculator,
/// 49 in a Slint app, 368 in VS Code; each one is a round trip to the program.
const LIMIT: usize = 10_000;
/// Chromium, and so every Electron app, shows its page to Accessibility only once asked with this.
/// Measured on VS Code 1.143.1: 13 elements before, 368 about 2.1 s after; it stays on until the program quits,
/// yet reads back false, so the page itself is what tells it was asked.
const CHROMIUM_TREE: &str = "AXManualAccessibility";
const PAGE: &str = "AXWebArea";
const TREE_POLL: Duration = Duration::from_millis(100);
/// A tree that stops growing for this long has arrived.
const TREE_SETTLE: Duration = Duration::from_millis(300);
const TREE_ANSWER: Duration = Duration::from_secs(5);
/// Elements whose children show only a part of themselves.
const CLIPPING: [&str; 2] = ["AXScrollArea", "AXWebArea"];

const ROLE: usize = 0;
const TITLE: usize = 1;
const DESCRIPTION: usize = 2;
const VALUE: usize = 3;
const ENABLED: usize = 4;
const FOCUSED: usize = 5;
const SELECTED: usize = 6;
const EXPANDED: usize = 7;
const POSITION: usize = 8;
const SIZE: usize = 9;
const CHILDREN: usize = 10;
const ATTRIBUTES: [&str; 11] = [
    "AXRole",
    "AXTitle",
    "AXDescription",
    "AXValue",
    "AXEnabled",
    "AXFocused",
    "AXSelected",
    "AXExpanded",
    "AXPosition",
    "AXSize",
    "AXChildren",
];

type GetWindow = unsafe extern "C" fn(*const AXUIElement, *mut u32) -> i32;

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(library: *mut c_void, name: *const c_char) -> *mut c_void;
}

/// The private call that ties an Accessibility window to its window id, which nothing public does.
fn get_window() -> Option<GetWindow> {
    static CALL: OnceLock<Option<GetWindow>> = OnceLock::new();
    *CALL.get_or_init(|| {
        // SAFETY: both strings are valid; a null handle or symbol is checked before use.
        unsafe {
            let library = dlopen(APPLICATION_SERVICES.as_ptr(), RTLD_LAZY);
            if library.is_null() {
                return None;
            }
            let call = dlsym(library, c"_AXUIElementGetWindow".as_ptr());
            // SAFETY: the symbol is a function with this signature on every macOS that has it.
            (!call.is_null()).then(|| transmute::<*mut c_void, GetWindow>(call))
        }
    })
}

fn window_id(element: &Element) -> Option<u32> {
    let call = get_window()?;
    let mut id = 0;
    let element: *const AXUIElement = &*element.0;
    // SAFETY: the element is live and the out pointer is valid.
    let error = unsafe { call(element, &mut id) };
    (error == 0).then_some(id)
}

/// The Accessibility window with the window's own id; never the nearest one.
fn described(app: &Element, window: &Window) -> Result<Element> {
    app.elements("AXWindows")
        .into_iter()
        .find(|found| window_id(found) == Some(window.id))
        .ok_or(Error::NotDescribed { pid: window.pid, window: window.id })
}

/// How many elements the window holds, up to just past [`LIMIT`], and whether a page is among them.
fn survey(window: &Element) -> (usize, bool) {
    let mut stack = window.elements("AXChildren");
    let (mut count, mut page) = (0, false);
    while let Some(node) = stack.pop() {
        count += 1;
        if count > LIMIT {
            break;
        }
        let mut values = node.attributes(&["AXRole", "AXChildren"]);
        page |= values[0].as_deref().and_then(text).as_deref() == Some(PAGE);
        stack.extend(values[1].take().map(element::elements).unwrap_or_default());
    }
    (count, page)
}

/// Asks a Chromium program for its page and waits for the tree to arrive. Other programs do not know the
/// attribute, so nothing changes for them; a window already showing its page is left alone.
fn ask_for_page(app: &Element, window: &Window) -> Result<()> {
    let Some(asked) = app.attribute(CHROMIUM_TREE) else {
        return Ok(());
    };
    let on = asked
        .downcast_ref::<CFBoolean>()
        .map(CFBoolean::as_bool)
        .or_else(|| asked.downcast_ref::<CFNumber>().and_then(CFNumber::as_i64).map(|n| n != 0));
    if on != Some(false) || survey(&described(app, window)?).1 || !app.switch_on(CHROMIUM_TREE) {
        return Ok(());
    }
    let started = Instant::now();
    let (mut last, mut since) = (0, Instant::now());
    while started.elapsed() < TREE_ANSWER {
        thread::sleep(TREE_POLL);
        let (count, page) = survey(&described(app, window)?);
        if count != last {
            (last, since) = (count, Instant::now());
        } else if page && since.elapsed() >= TREE_SETTLE {
            break;
        }
    }
    Ok(())
}

fn text(value: &CFType) -> Option<String> {
    let text = if let Some(text) = value.downcast_ref::<CFString>() {
        text.to_string()
    } else if let Some(flag) = value.downcast_ref::<CFBoolean>() {
        flag.as_bool().to_string()
    } else {
        let number = value.downcast_ref::<CFNumber>()?;
        match number.as_i64().filter(|_| !number.is_float_type()) {
            Some(whole) => whole.to_string(),
            None => number.as_f64()?.to_string(),
        }
    };
    (!text.is_empty()).then_some(text)
}

fn flag(value: &Option<CFRetained<CFType>>) -> Option<bool> {
    value.as_deref()?.downcast_ref::<CFBoolean>().map(CFBoolean::as_bool)
}

fn holds(frame: CGRect, x: f64, y: f64) -> bool {
    x >= frame.origin.x
        && y >= frame.origin.y
        && x < frame.origin.x + frame.size.width
        && y < frame.origin.y + frame.size.height
}

fn overlap(a: CGRect, b: CGRect) -> CGRect {
    let (left, top) = (a.origin.x.max(b.origin.x), a.origin.y.max(b.origin.y));
    let right = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let bottom = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);
    CGRect::new(CGPoint::new(left, top), CGSize::new((right - left).max(0.0), (bottom - top).max(0.0)))
}

/// One element as a control, or `None` for one not loaded yet or with nowhere to click.
fn control(
    values: &[Option<CFRetained<CFType>>],
    frame: Option<CGRect>,
    shown: CGRect,
    window: &Window,
) -> Option<Control> {
    let read = |index: usize| values[index].as_deref().and_then(text);
    let role = read(ROLE)?;
    let frame = frame?;
    let value = read(VALUE);
    let checked = match (role.as_str(), value.as_deref()) {
        ("AXCheckBox" | "AXRadioButton", Some("0")) => Some(false),
        ("AXCheckBox" | "AXRadioButton", Some("1")) => Some(true),
        _ => None,
    };
    let centre = (frame.origin.x + frame.size.width / 2.0, frame.origin.y + frame.size.height / 2.0);
    Some(Control {
        name: read(TITLE).or_else(|| read(DESCRIPTION)),
        value,
        enabled: flag(&values[ENABLED]),
        focused: flag(&values[FOCUSED]),
        selected: flag(&values[SELECTED]),
        checked,
        expanded: flag(&values[EXPANDED]),
        frame: Frame {
            x: frame.origin.x - window.x,
            y: frame.origin.y - window.y,
            width: frame.size.width,
            height: frame.size.height,
        },
        visible: holds(shown, centre.0, centre.1),
        role,
    })
}

/// Every element under the window, in the order the program lists them, the window itself left out.
pub(crate) fn read(window: &Window) -> Result<Vec<Control>> {
    // SAFETY: the system call takes no pointers.
    if !unsafe { AXIsProcessTrusted() } {
        return Err(Error::NoAccessibility);
    }
    // SAFETY: any process id is accepted; a wrong one yields an element with no attributes.
    let app = Element(unsafe { AXUIElement::new_application(window.pid) });
    ask_for_page(&app, window)?;
    let root = described(&app, window)?;
    let bounds = CGRect::new(CGPoint::new(window.x, window.y), CGSize::new(window.width, window.height));
    let mut stack: Vec<_> = root.elements("AXChildren").into_iter().rev().map(|child| (child, bounds)).collect();
    let mut controls = vec![];
    let mut seen = 0;
    while let Some((element, shown)) = stack.pop() {
        seen += 1;
        if seen > LIMIT {
            return Err(Error::TooManyControls { window: window.id, limit: LIMIT });
        }
        let mut values = element.attributes(&ATTRIBUTES);
        let frame = values[POSITION].take().zip(values[SIZE].take()).and_then(|(at, size)| element::frame(at, size));
        let role = values[ROLE].as_deref().and_then(text);
        let inner = match (role.as_deref(), frame) {
            (Some(role), Some(frame)) if CLIPPING.contains(&role) => overlap(shown, frame),
            _ => shown,
        };
        let children = values[CHILDREN].take().map(element::elements).unwrap_or_default();
        stack.extend(children.into_iter().rev().map(|child| (child, inner)));
        controls.extend(control(&values, frame, shown, window));
    }
    Ok(controls)
}
