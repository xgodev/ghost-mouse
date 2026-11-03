//! Finds, through Accessibility, the elements of a window that sit under one of its points.

use std::{
    thread,
    time::{Duration, Instant},
};

use ghost_mouse_core::{Point, Window};
use objc2_application_services::{AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::CGRect;

use crate::element::Element;

const TREE_POLL: Duration = Duration::from_millis(50);
/// How long a program gets to load what it shows Accessibility; a web view does it only once asked.
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

/// The window as Accessibility sees it.
pub(crate) fn element_of(window: &Window) -> Option<Element> {
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

/// The elements under a window point, from the window down to the innermost one. Empty when
/// Accessibility is not allowed or the window is not found.
pub(crate) fn path_at(window: &Window, at: Point) -> Vec<Element> {
    // SAFETY: the system call takes no pointers.
    if !unsafe { AXIsProcessTrusted() } {
        return vec![];
    }
    let started = Instant::now();
    loop {
        let Some(found) = element_of(window) else {
            return vec![];
        };
        let mut path = vec![];
        if path_to(found, window.x + at.x, window.y + at.y, &mut path) || started.elapsed() >= TREE_ANSWER {
            return path;
        }
        thread::sleep(TREE_POLL);
    }
}
