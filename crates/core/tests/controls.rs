//! Controls are found by what the program calls them, never guessed: one match is clicked at its centre,
//! anything else fails with what was found.

use std::{
    cell::{Cell, RefCell},
    path::Path,
    time::Duration,
};

use ghost_mouse_core::{Backend, Button, Control, Error, Frame, Ghost, Input, Launch, Point, Query, Result, Window};

const NO_WAIT: Duration = Duration::ZERO;

#[derive(Default)]
struct Fake {
    controls: Vec<Control>,
    /// `controls` answers empty until this many waits have passed.
    shows_after_waits: u32,
    waits: Cell<u32>,
    waited: Cell<Duration>,
    sent: RefCell<Vec<Input>>,
    /// The system can press what sits at any point.
    pressable: bool,
    pressed: RefCell<Vec<Point>>,
}

impl Backend for Fake {
    fn windows(&self, _pid: Option<i32>) -> Result<Vec<Window>> {
        Ok(vec![window()])
    }
    fn send(&self, _window: &Window, input: &Input) -> Result<()> {
        self.sent.borrow_mut().push(input.clone());
        Ok(())
    }
    fn wait(&self, time: Duration) {
        self.waited.set(self.waited.get() + time);
        self.waits.set(self.waits.get() + 1);
    }
    fn capture(&self, _window: &Window, _out: &Path) -> Result<()> {
        Ok(())
    }
    fn launch(&self, _launch: &Launch) -> Result<i32> {
        Ok(7)
    }
    fn front_pid(&self) -> Option<i32> {
        None
    }
    fn activate(&self, _pid: i32) -> Result<()> {
        Ok(())
    }
    fn press_at(&self, _window: &Window, at: Point) -> bool {
        if self.pressable {
            self.pressed.borrow_mut().push(at);
        }
        self.pressable
    }
    fn controls(&self, _window: &Window) -> Result<Vec<Control>> {
        if self.waits.get() < self.shows_after_waits {
            return Ok(vec![]);
        }
        Ok(self.controls.clone())
    }
}

/// A backend written before controls existed.
struct Old;

impl Backend for Old {
    fn windows(&self, _pid: Option<i32>) -> Result<Vec<Window>> {
        Ok(vec![window()])
    }
    fn send(&self, _window: &Window, _input: &Input) -> Result<()> {
        Ok(())
    }
    fn wait(&self, _time: Duration) {}
    fn capture(&self, _window: &Window, _out: &Path) -> Result<()> {
        Ok(())
    }
    fn launch(&self, _launch: &Launch) -> Result<i32> {
        Ok(7)
    }
    fn front_pid(&self) -> Option<i32> {
        None
    }
    fn activate(&self, _pid: i32) -> Result<()> {
        Ok(())
    }
}

fn window() -> Window {
    Window {
        id: 3,
        pid: 7,
        owner: "app".into(),
        title: "w".into(),
        x: -500.0,
        y: 40.0,
        width: 400.0,
        height: 300.0,
        on_screen: true,
        layer: 0,
        held_back: false,
    }
}

fn control(role: &str, name: Option<&str>, x: f64, y: f64) -> Control {
    Control {
        role: role.into(),
        name: name.map(Into::into),
        value: None,
        enabled: Some(true),
        focused: None,
        selected: None,
        checked: None,
        expanded: None,
        frame: Frame { x, y, width: 40.0, height: 20.0 },
        visible: true,
    }
}

fn named(name: &str) -> Query {
    Query { name: Some(name.into()), role: None }
}

fn calculator() -> Vec<Control> {
    vec![
        control("AXGroup", None, 0.0, 0.0),
        control("AXButton", Some("7"), 10.0, 100.0),
        control("AXStaticText", Some("7"), 10.0, 20.0),
        control("AXButton", Some("8"), 60.0, 100.0),
        control("AXButton", Some("Start"), 10.0, 150.0),
        control("AXButton", Some("Start"), 60.0, 150.0),
    ]
}

#[test]
fn a_frame_centre_is_halfway_across_and_down() {
    let frame = Frame { x: 10.0, y: 187.0, width: 48.0, height: 48.0 };
    assert_eq!(frame.center(), Point { x: 34.0, y: 211.0 });
}

#[test]
fn listing_keeps_only_what_matches_name_and_role() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let w = window();

    assert_eq!(ghost.controls(&w, &Query::default(), NO_WAIT).unwrap().len(), 6);
    let sevens = ghost.controls(&w, &named("7"), NO_WAIT).unwrap();
    assert_eq!(sevens.iter().map(|c| c.role.as_str()).collect::<Vec<_>>(), ["AXButton", "AXStaticText"]);
    let button = Query { name: Some("7".into()), role: Some("AXButton".into()) };
    assert_eq!(ghost.controls(&w, &button, NO_WAIT).unwrap(), [calculator()[1].clone()]);
    assert!(ghost.controls(&w, &named("Seven"), NO_WAIT).unwrap().is_empty());
}

#[test]
fn a_name_matches_whole_and_exact() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    assert!(ghost.controls(&window(), &named("Star"), NO_WAIT).unwrap().is_empty());
    assert!(ghost.controls(&window(), &named("start"), NO_WAIT).unwrap().is_empty());
}

#[test]
fn listing_waits_until_something_matches() {
    let ghost = Ghost::new(Fake { controls: calculator(), shows_after_waits: 3, ..Fake::default() });
    let found = ghost.controls(&window(), &named("8"), Duration::from_secs(5)).unwrap();
    assert_eq!(found.len(), 1);
    assert!(ghost.backend().waited.get() < Duration::from_secs(5));
}

#[test]
fn listing_with_nothing_matching_ends_empty_once_the_wait_is_over() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let wait = Duration::from_secs(2);
    assert!(ghost.controls(&window(), &named("Nope"), wait).unwrap().is_empty());
    assert!(ghost.backend().waited.get() >= wait);
}

#[test]
fn a_click_by_name_lands_on_the_centre_after_resting_there() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let clicked = ghost.click_control(&window(), &named("8"), None, Button::Left, 1, NO_WAIT).unwrap();
    assert_eq!(clicked.name.as_deref(), Some("8"));
    let centre = Point { x: 80.0, y: 110.0 };
    let sent = ghost.backend().sent.borrow().clone();
    assert_eq!(sent.first(), Some(&Input::MouseMove { at: centre }));
    assert!(sent.contains(&Input::MouseDown { at: centre, button: Button::Left, clicks: 1 }), "{sent:?}");
    assert!(sent.contains(&Input::MouseUp { at: centre, button: Button::Left, clicks: 1 }), "{sent:?}");
}

#[test]
fn a_click_by_name_goes_the_way_a_click_at_a_point_goes() {
    let ghost = Ghost::new(Fake { controls: calculator(), pressable: true, ..Fake::default() });
    ghost.click_control(&window(), &named("8"), None, Button::Left, 1, NO_WAIT).unwrap();
    assert_eq!(*ghost.backend().pressed.borrow(), [Point { x: 80.0, y: 110.0 }]);
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn a_hover_by_name_rests_on_the_centre() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    ghost.hover_control(&window(), &named("8"), None, NO_WAIT).unwrap();
    assert_eq!(*ghost.backend().sent.borrow(), [Input::MouseMove { at: Point { x: 80.0, y: 110.0 } }]);
}

#[test]
fn a_click_by_name_waits_for_the_control_to_show() {
    let ghost = Ghost::new(Fake { controls: calculator(), shows_after_waits: 2, ..Fake::default() });
    ghost.click_control(&window(), &named("8"), None, Button::Left, 1, Duration::from_secs(5)).unwrap();
    assert!(ghost.backend().sent.borrow().contains(&Input::MouseDown {
        at: Point { x: 80.0, y: 110.0 },
        button: Button::Left,
        clicks: 1
    }));
}

#[test]
fn nothing_matching_fails_with_the_named_controls_found() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let error = ghost.click_control(&window(), &named("Nope"), None, Button::Left, 1, NO_WAIT).unwrap_err();
    let Error::NoControl { candidates, .. } = &error else { panic!("{error}") };
    assert_eq!(candidates.len(), 5, "{candidates:?}");
    assert!(candidates.iter().any(|c| c.contains("AXButton") && c.contains("\"8\"")), "{candidates:?}");
    assert!(error.to_string().contains("\"Start\""), "{error}");
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn a_long_list_of_candidates_is_cut_and_says_how_many_were_left_out() {
    let controls = (0..100).map(|n| control("AXButton", Some(&n.to_string()), 0.0, 0.0)).collect();
    let ghost = Ghost::new(Fake { controls, ..Fake::default() });
    let error = ghost.find_control(&window(), &named("Nope"), None, NO_WAIT).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("\"0\""), "{text}");
    assert!(!text.contains("\"99\""), "{text}");
    assert!(text.contains("more"), "{text}");
}

#[test]
fn several_matches_fail_with_each_one_found() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let error = ghost.click_control(&window(), &named("Start"), None, Button::Left, 1, NO_WAIT).unwrap_err();
    let Error::ManyControls { found, .. } = &error else { panic!("{error}") };
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("10,150") && found[1].contains("60,150"), "{found:?}");
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn nth_picks_one_of_several_in_listing_order_counting_from_one() {
    let ghost = Ghost::new(Fake { controls: calculator(), pressable: true, ..Fake::default() });
    let picked = ghost.click_control(&window(), &named("Start"), Some(2), Button::Left, 1, NO_WAIT).unwrap();
    assert_eq!(picked.frame.x, 60.0);
    assert_eq!(*ghost.backend().pressed.borrow(), [Point { x: 80.0, y: 160.0 }]);
}

#[test]
fn nth_past_the_matches_is_refused() {
    let ghost = Ghost::new(Fake { controls: calculator(), ..Fake::default() });
    let error = ghost.find_control(&window(), &named("Start"), Some(3), NO_WAIT).unwrap_err();
    assert!(matches!(error, Error::NoNth { nth: 3, count: 2, .. }), "{error}");
    let error = ghost.find_control(&window(), &named("Start"), Some(0), NO_WAIT).unwrap_err();
    assert!(matches!(error, Error::Invalid { .. }), "{error}");
}

#[test]
fn a_control_out_of_view_is_refused_so_the_caller_scrolls_first() {
    let mut far = control("AXButton", Some("Far"), 10.0, 100.0);
    far.visible = false;
    let ghost = Ghost::new(Fake { controls: vec![far], ..Fake::default() });
    let error = ghost.click_control(&window(), &named("Far"), None, Button::Left, 1, NO_WAIT).unwrap_err();
    assert!(matches!(error, Error::ControlHidden { .. }), "{error}");
    assert!(error.to_string().contains("scroll"), "{error}");
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn a_control_whose_centre_is_past_the_window_edge_is_refused() {
    let below = control("AXButton", Some("Below"), 10.0, 400.0);
    let ghost = Ghost::new(Fake { controls: vec![below], ..Fake::default() });
    let error = ghost.hover_control(&window(), &named("Below"), None, NO_WAIT).unwrap_err();
    assert!(matches!(error, Error::ControlHidden { .. }), "{error}");
}

#[test]
fn a_system_without_controls_says_so() {
    let ghost = Ghost::new(Old);
    let error = ghost.controls(&window(), &Query::default(), NO_WAIT).unwrap_err();
    assert!(matches!(error, Error::Unsupported), "{error}");
}

#[test]
fn a_control_is_listed_as_json_without_the_states_the_program_does_not_give() {
    let mut seven = control("AXButton", Some("7"), 10.0, 187.0);
    seven.checked = Some(false);
    let json = serde_json::to_value(&seven).unwrap();
    assert_eq!(json["role"], "AXButton");
    assert_eq!(json["name"], "7");
    assert_eq!(json["checked"], false);
    assert_eq!(json["frame"]["x"], 10.0);
    assert_eq!(json["visible"], true);
    assert!(json.get("value").is_none() && json.get("focused").is_none(), "{json}");
}
