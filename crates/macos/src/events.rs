//! Builds each event and delivers it straight to one process.

use ghost_mouse_core::{Button, Chord, Error, Input, Point, Result, Window};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_core_foundation::{CFRetained, CGPoint};
use objc2_core_graphics::{CGDisplayBounds, CGEvent, CGEventField, CGEventFlags, CGEventType, CGMainDisplayID};
use objc2_foundation::NSProcessInfo;

use crate::{location::event_location, virtual_keys::virtual_key};

const PIXELS_PER_LINE: i64 = 10;
const MIDDLE_BUTTON_NUMBER: i64 = 2;

/// Delivers one event to the window's process. The real cursor and front app are not involved.
pub(crate) fn post(window: &Window, input: &Input) -> Result<()> {
    let event = build(window, input)?;
    CGEvent::post_to_pid(window.pid, Some(&event));
    Ok(())
}

/// Returns where the real cursor is, in desktop points.
pub fn cursor_position() -> Option<Point> {
    let event = CGEvent::new(None)?;
    let at = CGEvent::location(Some(&event));
    Some(Point { x: at.x, y: at.y })
}

fn build(window: &Window, input: &Input) -> Result<CFRetained<CGEvent>> {
    match input {
        Input::MouseMove { at } => mouse(window, NSEventType::MouseMoved, *at, 0, 0.0, Button::Left),
        Input::MouseDown { at, button, clicks } => mouse(window, down_type(*button), *at, *clicks, 1.0, *button),
        Input::MouseUp { at, button, clicks } => mouse(window, up_type(*button), *at, *clicks, 0.0, *button),
        Input::MouseDrag { at, button } => mouse(window, drag_type(*button), *at, 1, 1.0, *button),
        Input::Scroll { at, dx, dy } => scroll(window, *at, *dx, *dy),
        Input::Key { chord, down } => key(*chord, *down),
        Input::Text { ch, down } => text(*ch, *down),
    }
}

fn down_type(button: Button) -> NSEventType {
    match button {
        Button::Left => NSEventType::LeftMouseDown,
        Button::Right => NSEventType::RightMouseDown,
        Button::Middle => NSEventType::OtherMouseDown,
    }
}

fn up_type(button: Button) -> NSEventType {
    match button {
        Button::Left => NSEventType::LeftMouseUp,
        Button::Right => NSEventType::RightMouseUp,
        Button::Middle => NSEventType::OtherMouseUp,
    }
}

fn drag_type(button: Button) -> NSEventType {
    match button {
        Button::Left => NSEventType::LeftMouseDragged,
        Button::Right => NSEventType::RightMouseDragged,
        Button::Middle => NSEventType::OtherMouseDragged,
    }
}

/// Only an event built by AppKit with a window number reaches the view of a window in the
/// background; a plain CGEvent arrives with no window and is dropped.
fn mouse(
    window: &Window,
    kind: NSEventType,
    at: Point,
    clicks: u8,
    pressure: f32,
    button: Button,
) -> Result<CFRetained<CGEvent>> {
    let display_height = CGDisplayBounds(CGMainDisplayID()).size.height;
    let location = event_location(at, display_height);
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind,
        CGPoint { x: location.x, y: location.y },
        NSEventModifierFlags::empty(),
        NSProcessInfo::processInfo().systemUptime(),
        isize::try_from(window.id).map_err(|e| Error::Event(e.to_string()))?,
        None,
        0,
        isize::from(clicks),
        pressure,
    )
    .and_then(|event| event.CGEvent())
    .ok_or_else(|| Error::Event(format!("mouse event at {at}")))?;
    if button == Button::Middle {
        CGEvent::set_integer_value_field(Some(&event), CGEventField::MouseEventButtonNumber, MIDDLE_BUTTON_NUMBER);
    }
    Ok(event.into())
}

/// Starts from a mouse event so the scroll carries the window; AppKit has no scroll factory.
fn scroll(window: &Window, at: Point, dx: i32, dy: i32) -> Result<CFRetained<CGEvent>> {
    let event = mouse(window, NSEventType::MouseMoved, at, 0, 0.0, Button::Left)?;
    CGEvent::set_type(Some(&event), CGEventType::ScrollWheel);
    let (lines_y, lines_x) = (-i64::from(dy), -i64::from(dx));
    for (field, value) in [
        (CGEventField::ScrollWheelEventDeltaAxis1, lines_y),
        (CGEventField::ScrollWheelEventDeltaAxis2, lines_x),
        (CGEventField::ScrollWheelEventPointDeltaAxis1, lines_y * PIXELS_PER_LINE),
        (CGEventField::ScrollWheelEventPointDeltaAxis2, lines_x * PIXELS_PER_LINE),
    ] {
        CGEvent::set_integer_value_field(Some(&event), field, value);
    }
    Ok(event)
}

fn key(chord: Chord, down: bool) -> Result<CFRetained<CGEvent>> {
    let code = virtual_key(chord.key).ok_or_else(|| Error::Event(format!("no key code for {:?}", chord.key)))?;
    let event = CGEvent::new_keyboard_event(None, code, down).ok_or_else(|| Error::Event(format!("key {code}")))?;
    let mut flags = CGEventFlags::empty();
    for (held, flag) in [
        (chord.modifiers.shift, CGEventFlags::MaskShift),
        (chord.modifiers.control, CGEventFlags::MaskControl),
        (chord.modifiers.option, CGEventFlags::MaskAlternate),
        (chord.modifiers.command, CGEventFlags::MaskCommand),
    ] {
        if held {
            flags |= flag;
        }
    }
    CGEvent::set_flags(Some(&event), flags);
    Ok(event)
}

fn text(ch: char, down: bool) -> Result<CFRetained<CGEvent>> {
    let event = CGEvent::new_keyboard_event(None, 0, down).ok_or_else(|| Error::Event(format!("text {ch}")))?;
    let mut units = [0u16; 2];
    let encoded = ch.encode_utf16(&mut units);
    // SAFETY: the pointer and length describe the live UTF-16 buffer; the event copies it.
    unsafe { CGEvent::keyboard_set_unicode_string(Some(&event), encoded.len() as _, encoded.as_ptr()) };
    CGEvent::set_flags(Some(&event), CGEventFlags::empty());
    Ok(event)
}
