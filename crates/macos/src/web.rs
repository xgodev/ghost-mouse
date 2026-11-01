//! Clicks a web page through Accessibility, which reaches it behind other apps.
//!
//! A web view refuses the first click of a window that is not the key one, so a mouse event never gets
//! to the page, and a window cannot be made key without bringing it forward. Anything outside a web
//! page keeps going as events.

use std::{
    thread,
    time::{Duration, Instant},
};

use ghost_mouse_core::{Point, Window};
use objc2_application_services::{AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::CGRect;

use crate::element::Element;

const WEB_PAGE: &str = "AXWebArea";
const TREE_POLL: Duration = Duration::from_millis(50);
/// How long a web view gets to load what it shows Accessibility, which it does only once asked.
const TREE_ANSWER: Duration = Duration::from_secs(2);

fn holds(frame: CGRect, x: f64, y: f64) -> bool {
    x >= frame.origin.x
        && y >= frame.origin.y
        && x < frame.origin.x + frame.size.width
        && y < frame.origin.y + frame.size.height
}

/// A window picked just after opening may still be settling, so its frame is not compared exactly.
fn distance(frame: CGRect, window: &Window) -> f64 {
    (frame.origin.x - window.x).abs()
        + (frame.origin.y - window.y).abs()
        + (frame.size.width - window.width).abs()
        + (frame.size.height - window.height).abs()
}

fn element_of(window: &Window) -> Option<Element> {
    // SAFETY: any process id is accepted; a wrong one yields an element with no attributes.
    let app = Element(unsafe { AXUIElement::new_application(window.pid) });
    let mut found: Vec<_> =
        app.elements("AXWindows").into_iter().filter_map(|w| Some((distance(w.frame()?, window), w))).collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().next().map(|(_, element)| element)
}

/// The elements under a desktop point, outermost first. The last child drawn is the one on top.
/// `false` when a child on the way has not loaded yet, which shows as having no role.
fn path_to(element: Element, x: f64, y: f64, path: &mut Vec<Element>) -> bool {
    let mut children = element.elements("AXChildren");
    path.push(element);
    if children.iter().any(|child| child.text("AXRole").is_none()) {
        return false;
    }
    children.reverse();
    match children.into_iter().find(|child| child.frame().is_some_and(|frame| holds(frame, x, y))) {
        Some(child) => path_to(child, x, y, path),
        None => true,
    }
}

/// Presses what the page shows at the point, or the nearest thing around it that can be pressed.
/// `false` when the point is not on a web page, or nothing there takes a press.
pub(crate) fn press(window: &Window, at: Point) -> bool {
    // SAFETY: the system call takes no pointers.
    if !unsafe { AXIsProcessTrusted() } {
        return false;
    }
    let started = Instant::now();
    let path = loop {
        let Some(found) = element_of(window) else {
            return false;
        };
        let mut path = vec![];
        if path_to(found, window.x + at.x, window.y + at.y, &mut path) || started.elapsed() >= TREE_ANSWER {
            break path;
        }
        thread::sleep(TREE_POLL);
    };
    let Some(page) = path.iter().position(|element| element.is(WEB_PAGE)) else {
        return false;
    };
    path[page + 1..].iter().rev().any(Element::press)
}
