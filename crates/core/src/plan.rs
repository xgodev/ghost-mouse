//! Turns one gesture into the ordered events a real hand would produce.

use std::time::Duration;

use crate::{
    geometry::Point,
    input::{Button, Chord, Input},
};

/// Gap between events of one gesture, so programs that sample their input see each state.
pub const GAP: Duration = Duration::from_millis(40);

/// One thing to do while performing a gesture.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Send(Input),
    Wait(Duration),
}

/// Rests the pointer over a point.
pub fn hover(at: Point) -> Vec<Step> {
    vec![Step::Send(Input::MouseMove { at }), Step::Wait(GAP)]
}

/// Moves to the point and presses `clicks` times; the click count rises on each press.
pub fn click(at: Point, button: Button, clicks: u8) -> Vec<Step> {
    let mut steps = hover(at);
    for clicks in 1..=clicks {
        steps.extend([
            Step::Send(Input::MouseDown { at, button, clicks }),
            Step::Wait(GAP),
            Step::Send(Input::MouseUp { at, button, clicks }),
            Step::Wait(GAP),
        ]);
    }
    steps
}

/// Presses at `from`, passes through `steps` evenly spaced points and releases at `to`.
pub fn drag(from: Point, to: Point, button: Button, steps: u32) -> Vec<Step> {
    let count = steps.max(1);
    let mut plan = hover(from);
    plan.extend([Step::Send(Input::MouseDown { at: from, button, clicks: 1 }), Step::Wait(GAP)]);
    for step in 1..=count {
        let done = f64::from(step) / f64::from(count);
        let at = Point { x: from.x + (to.x - from.x) * done, y: from.y + (to.y - from.y) * done };
        plan.extend([Step::Send(Input::MouseDrag { at, button }), Step::Wait(GAP)]);
    }
    plan.push(Step::Send(Input::MouseUp { at: to, button, clicks: 1 }));
    plan.push(Step::Wait(GAP));
    plan
}

/// Moves to the point and scrolls.
pub fn scroll(at: Point, dx: i32, dy: i32) -> Vec<Step> {
    let mut steps = hover(at);
    steps.extend([Step::Send(Input::Scroll { at, dx, dy }), Step::Wait(GAP)]);
    steps
}

/// Presses and releases one key combination.
pub fn press(chord: Chord) -> Vec<Step> {
    vec![
        Step::Send(Input::Key { chord, down: true }),
        Step::Wait(GAP),
        Step::Send(Input::Key { chord, down: false }),
        Step::Wait(GAP),
    ]
}

/// Types the text one character at a time.
pub fn type_text(text: &str) -> Vec<Step> {
    text.chars()
        .flat_map(|ch| {
            [Step::Send(Input::Text { ch, down: true }), Step::Send(Input::Text { ch, down: false }), Step::Wait(GAP)]
        })
        .collect()
}
