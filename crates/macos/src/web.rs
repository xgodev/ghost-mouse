//! Clicks a web page through Accessibility, which reaches it behind other apps.
//!
//! A web view refuses the first click of a window that is not the key one, so a mouse event never gets
//! to the page, and a window cannot be made key without bringing it forward. Anything outside a web
//! page keeps going as events.

use crate::element::Element;

const WEB_PAGE: &str = "AXWebArea";

/// Presses what the page shows under a point, given the elements there outermost first, or the
/// nearest thing around it that can be pressed. `false` when the point is not on a web page, or
/// nothing there takes a press.
pub(crate) fn press(path: &[Element]) -> bool {
    let Some(page) = path.iter().position(|element| element.is(WEB_PAGE)) else {
        return false;
    };
    path[page + 1..].iter().rev().any(Element::press)
}
