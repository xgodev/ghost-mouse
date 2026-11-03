//! Clicks and scrolls an iPad app running on the Mac (Mac Catalyst) through Accessibility.
//!
//! Measured on macOS 26: UIKit drops every mouse and scroll event that reaches a window of a program
//! not in front, even with the window number on it, and an Accessibility press that UIKit turns into
//! a mouse event is dropped the same way. A press on a button or a list row works, a text field takes
//! the focus once its value is written, and a scrolling area takes a page up or down. Keys reach the
//! field once it has the focus.

use ghost_mouse_core::Window;

use crate::{element::Element, pointed::element_of};

const IPAD_CONTENT: &str = "iOSContentGroup";
const TEXT_ROLES: [&str; 3] = ["AXTextField", "AXTextArea", "AXComboBox"];

/// Whether the window shows an iPad app, read without walking its interface.
pub(crate) fn shows_ipad_content(window: &Window) -> bool {
    element_of(window).is_some_and(|window| {
        window.elements("AXChildren").iter().any(|child| child.text("AXSubrole").as_deref() == Some(IPAD_CONTENT))
    })
}

/// The elements of a path that are inside the iPad content; empty outside it.
fn content(path: &[Element]) -> &[Element] {
    match path.iter().position(|element| element.text("AXSubrole").as_deref() == Some(IPAD_CONTENT)) {
        Some(content) => &path[content + 1..],
        None => &[],
    }
}

/// UIKit ignores a plain request for the focus from a program in the background, but gives it when
/// the field's value is written; writing back the value it has leaves the text as it was.
fn focus(field: &Element) -> bool {
    if field.switch_on("AXFocused") && field.flag("AXFocused") {
        return true;
    }
    let Some(text) = field.text("AXValue") else {
        return false;
    };
    field.set_text("AXValue", &text) && field.flag("AXFocused")
}

/// Focuses the text field under a point, given the elements there outermost first, or presses the
/// nearest thing there that takes a press. `false` when the point is not on iPad content, or nothing
/// there reacts.
pub(crate) fn press(path: &[Element]) -> bool {
    let path = content(path);
    if let Some(field) = path.iter().rev().find(|element| TEXT_ROLES.iter().any(|role| element.is(role))) {
        return focus(field);
    }
    path.iter().rev().any(Element::press)
}

/// Moves the scrolling area under a point one page in the direction asked, vertical first, given the
/// elements there outermost first. `false` when the point is not on iPad content, or nothing there
/// scrolls that way.
pub(crate) fn scroll(path: &[Element], dx: i32, dy: i32) -> bool {
    let action = match (dx.signum(), dy.signum()) {
        (_, 1) => "AXScrollDownByPage",
        (_, -1) => "AXScrollUpByPage",
        (1, _) => "AXScrollRightByPage",
        (-1, _) => "AXScrollLeftByPage",
        _ => return false,
    };
    content(path)
        .iter()
        .rev()
        .find(|element| element.actions().iter().any(|name| name == action))
        .is_some_and(|element| element.perform(action))
}
