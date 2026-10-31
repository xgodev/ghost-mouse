//! Each gesture becomes the events a real hand would produce, in order.

use ghost_mouse_core::{
    plan::{self, Step},
    Button, Chord, Input, Key, Modifiers, Point,
};

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn inputs(steps: &[Step]) -> Vec<Input> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Send(input) => Some(input.clone()),
            Step::Wait(_) => None,
        })
        .collect()
}

#[test]
fn click_moves_there_then_presses_and_releases() {
    let p = at(10.0, 20.0);
    assert_eq!(
        inputs(&plan::click(p, Button::Left, 1)),
        vec![
            Input::MouseMove { at: p },
            Input::MouseDown { at: p, button: Button::Left, clicks: 1 },
            Input::MouseUp { at: p, button: Button::Left, clicks: 1 },
        ]
    );
}

#[test]
fn double_click_counts_up() {
    let p = at(1.0, 2.0);
    let events = inputs(&plan::click(p, Button::Right, 2));
    assert_eq!(
        events[1..],
        [
            Input::MouseDown { at: p, button: Button::Right, clicks: 1 },
            Input::MouseUp { at: p, button: Button::Right, clicks: 1 },
            Input::MouseDown { at: p, button: Button::Right, clicks: 2 },
            Input::MouseUp { at: p, button: Button::Right, clicks: 2 },
        ]
    );
}

#[test]
fn press_and_release_are_never_back_to_back() {
    let steps = plan::click(at(1.0, 2.0), Button::Left, 1);
    let down = steps.iter().position(|s| matches!(s, Step::Send(Input::MouseDown { .. }))).unwrap();
    assert!(matches!(steps[down + 1], Step::Wait(_)));
}

#[test]
fn drag_presses_at_the_start_and_releases_at_the_end() {
    let (from, to) = (at(0.0, 0.0), at(100.0, 50.0));
    let events = inputs(&plan::drag(from, to, Button::Left, 4));
    assert_eq!(
        events,
        vec![
            Input::MouseMove { at: from },
            Input::MouseDown { at: from, button: Button::Left, clicks: 1 },
            Input::MouseDrag { at: at(25.0, 12.5), button: Button::Left },
            Input::MouseDrag { at: at(50.0, 25.0), button: Button::Left },
            Input::MouseDrag { at: at(75.0, 37.5), button: Button::Left },
            Input::MouseDrag { at: to, button: Button::Left },
            Input::MouseUp { at: to, button: Button::Left, clicks: 1 },
        ]
    );
}

#[test]
fn drag_with_zero_steps_still_reaches_the_end() {
    let events = inputs(&plan::drag(at(0.0, 0.0), at(9.0, 9.0), Button::Left, 0));
    assert!(events.contains(&Input::MouseDrag { at: at(9.0, 9.0), button: Button::Left }));
}

#[test]
fn scroll_moves_there_first() {
    let p = at(5.0, 6.0);
    assert_eq!(inputs(&plan::scroll(p, 0, 3)), vec![Input::MouseMove { at: p }, Input::Scroll { at: p, dx: 0, dy: 3 }]);
}

#[test]
fn hover_only_moves() {
    let p = at(5.0, 6.0);
    assert_eq!(inputs(&plan::hover(p)), vec![Input::MouseMove { at: p }]);
}

#[test]
fn press_goes_down_then_up() {
    let chord = Chord { key: Key::Return, modifiers: Modifiers::default() };
    assert_eq!(inputs(&plan::press(chord)), vec![Input::Key { chord, down: true }, Input::Key { chord, down: false }]);
}

#[test]
fn text_goes_one_character_at_a_time() {
    assert_eq!(
        inputs(&plan::type_text("aç")),
        vec![
            Input::Text { ch: 'a', down: true },
            Input::Text { ch: 'a', down: false },
            Input::Text { ch: 'ç', down: true },
            Input::Text { ch: 'ç', down: false },
        ]
    );
}
