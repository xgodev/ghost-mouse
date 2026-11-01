//! The operations pick the right window, refuse points that would hit nothing and run each gesture.

use std::{
    cell::{Cell, RefCell},
    fs,
    path::Path,
    time::Duration,
};

use ghost_mouse_core::{Backend, Button, Error, Ghost, Input, Launch, Point, Result, Window};

const PROGRAM: i32 = 7;
const OWNER_APP: i32 = 100;
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Fake {
    windows: Vec<Window>,
    /// `windows` answers empty this many times before showing the list.
    empty_answers: Cell<u32>,
    sent: RefCell<Vec<(u32, Input)>>,
    waited: Cell<Duration>,
    picture: Vec<u8>,
    /// The process in front right now.
    front: Cell<Option<i32>>,
    /// The opened program takes the front after this many waits; 0 is as soon as it starts.
    grabs_after_waits: Option<u32>,
    /// Someone else takes the front when the program starts.
    person_moves_to: Option<i32>,
    activate_fails: bool,
    activated: RefCell<Vec<i32>>,
    waits: Cell<u32>,
    /// The system can press what sits at any point.
    pressable: bool,
    pressed: RefCell<Vec<(u32, Point)>>,
}

impl Backend for Fake {
    fn press_at(&self, window: &Window, at: Point) -> bool {
        if self.pressable {
            self.pressed.borrow_mut().push((window.id, at));
        }
        self.pressable
    }
    fn windows(&self, pid: Option<i32>) -> Result<Vec<Window>> {
        if self.empty_answers.get() > 0 {
            self.empty_answers.set(self.empty_answers.get() - 1);
            return Ok(vec![]);
        }
        Ok(self.windows.iter().filter(|w| pid.is_none_or(|p| p == w.pid)).cloned().collect())
    }
    fn send(&self, window: &Window, input: &Input) -> Result<()> {
        self.sent.borrow_mut().push((window.id, input.clone()));
        Ok(())
    }
    fn wait(&self, time: Duration) {
        self.waited.set(self.waited.get() + time);
        self.waits.set(self.waits.get() + 1);
        if self.grabs_after_waits == Some(self.waits.get()) {
            self.front.set(Some(PROGRAM));
        }
    }
    fn capture(&self, _window: &Window, out: &Path) -> Result<()> {
        fs::write(out, &self.picture)?;
        Ok(())
    }
    fn launch(&self, _launch: &Launch) -> Result<i32> {
        if self.grabs_after_waits == Some(0) {
            self.front.set(Some(PROGRAM));
        }
        if let Some(person) = self.person_moves_to {
            self.front.set(Some(person));
        }
        Ok(PROGRAM)
    }
    fn front_pid(&self) -> Option<i32> {
        self.front.get()
    }
    fn activate(&self, pid: i32) -> Result<()> {
        if self.activate_fails {
            return Err(Error::Event("refused".into()));
        }
        self.activated.borrow_mut().push(pid);
        self.front.set(Some(pid));
        Ok(())
    }
}

fn window(id: u32, title: &str, width: f64, height: f64, on_screen: bool) -> Window {
    Window { id, pid: 7, owner: "app".into(), title: title.into(), x: -500.0, y: 40.0, width, height, on_screen }
}

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn a_click_the_system_can_press_sends_no_events() {
    let ghost = Ghost::new(Fake { pressable: true, ..Fake::default() });
    let target = window(1, "x", 100.0, 100.0, true);
    ghost.click(&target, at(10.0, 20.0), Button::Left, 1).unwrap();
    assert_eq!(*ghost.backend().pressed.borrow(), vec![(1, at(10.0, 20.0))]);
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn a_click_the_system_cannot_press_goes_as_events() {
    let ghost = Ghost::new(Fake::default());
    let target = window(1, "x", 100.0, 100.0, true);
    ghost.click(&target, at(10.0, 20.0), Button::Left, 1).unwrap();
    assert!(!ghost.backend().sent.borrow().is_empty());
}

#[test]
fn only_a_single_left_click_is_ever_pressed() {
    let ghost = Ghost::new(Fake { pressable: true, ..Fake::default() });
    let target = window(1, "x", 100.0, 100.0, true);
    ghost.click(&target, at(10.0, 20.0), Button::Left, 2).unwrap();
    ghost.click(&target, at(10.0, 20.0), Button::Right, 1).unwrap();
    assert!(ghost.backend().pressed.borrow().is_empty());
    assert!(!ghost.backend().sent.borrow().is_empty());
}

#[test]
fn a_point_outside_the_window_is_never_pressed() {
    let ghost = Ghost::new(Fake { pressable: true, ..Fake::default() });
    let target = window(1, "x", 100.0, 100.0, true);
    assert!(ghost.click(&target, at(100.0, 20.0), Button::Left, 1).is_err());
    assert!(ghost.backend().pressed.borrow().is_empty());
}

#[test]
fn main_window_is_the_visible_named_largest_one() {
    let ghost = Ghost::new(Fake {
        windows: vec![
            window(1, "", 2000.0, 2000.0, true),
            window(2, "hidden", 3000.0, 3000.0, false),
            window(3, "small", 100.0, 100.0, true),
            window(4, "main", 800.0, 600.0, true),
            window(5, "zero", 0.0, 0.0, true),
        ],
        ..Fake::default()
    });
    assert_eq!(ghost.main_window(7).unwrap().id, 4);
}

#[test]
fn main_window_accepts_an_unnamed_window_when_nothing_else_exists() {
    let ghost = Ghost::new(Fake {
        windows: vec![window(1, "", 800.0, 600.0, true), window(2, "", 10.0, 10.0, true)],
        ..Fake::default()
    });
    assert_eq!(ghost.main_window(7).unwrap().id, 1);
}

#[test]
fn main_window_fails_when_nothing_is_visible() {
    let ghost = Ghost::new(Fake { windows: vec![window(1, "x", 10.0, 10.0, false)], ..Fake::default() });
    assert!(matches!(ghost.main_window(7), Err(Error::NoWindow { pid: 7 })));
    assert!(matches!(ghost.main_window(8), Err(Error::NoWindow { pid: 8 })));
}

#[test]
fn window_finds_by_id_within_the_process() {
    let ghost = Ghost::new(Fake { windows: vec![window(9, "x", 10.0, 10.0, true)], ..Fake::default() });
    assert_eq!(ghost.window(7, 9).unwrap().id, 9);
    assert!(matches!(ghost.window(7, 1), Err(Error::UnknownWindow { pid: 7, window: 1 })));
    assert!(matches!(ghost.window(8, 9), Err(Error::UnknownWindow { pid: 8, window: 9 })));
}

#[test]
fn wait_for_window_returns_as_soon_as_it_appears() {
    let ghost = Ghost::new(Fake {
        windows: vec![window(1, "x", 10.0, 10.0, true)],
        empty_answers: Cell::new(3),
        ..Fake::default()
    });
    assert_eq!(ghost.wait_for_window(7, Duration::from_secs(5)).unwrap().id, 1);
    assert!(ghost.backend().waited.get() < Duration::from_secs(5));
}

#[test]
fn wait_for_window_gives_up_at_the_deadline() {
    let ghost = Ghost::new(Fake::default());
    assert!(matches!(ghost.wait_for_window(7, Duration::from_secs(1)), Err(Error::NoWindow { pid: 7 })));
    assert!(ghost.backend().waited.get() >= Duration::from_secs(1));
}

#[test]
fn click_reaches_only_the_chosen_window() {
    let target = window(4, "main", 800.0, 600.0, true);
    let ghost = Ghost::new(Fake::default());
    ghost.click(&target, at(10.0, 20.0), Button::Left, 1).unwrap();
    let sent = ghost.backend().sent.borrow();
    assert_eq!(sent.len(), 3);
    assert!(sent.iter().all(|(id, _)| *id == 4));
}

#[test]
fn points_outside_the_window_send_nothing() {
    let target = window(4, "main", 800.0, 600.0, true);
    let ghost = Ghost::new(Fake::default());
    for bad in [at(800.0, 10.0), at(10.0, 600.0), at(-1.0, 10.0), at(10.0, -1.0)] {
        assert!(matches!(ghost.click(&target, bad, Button::Left, 1), Err(Error::OutsideWindow { .. })), "{bad}");
        assert!(matches!(ghost.hover(&target, bad), Err(Error::OutsideWindow { .. })), "{bad}");
        assert!(matches!(ghost.scroll(&target, bad, 0, 1), Err(Error::OutsideWindow { .. })), "{bad}");
        assert!(ghost.drag(&target, bad, at(1.0, 1.0), Button::Left, 2).is_err(), "{bad}");
        assert!(ghost.drag(&target, at(1.0, 1.0), bad, Button::Left, 2).is_err(), "{bad}");
    }
    assert!(ghost.backend().sent.borrow().is_empty());
}

#[test]
fn zero_clicks_are_refused() {
    let target = window(4, "main", 800.0, 600.0, true);
    let ghost = Ghost::new(Fake::default());
    assert!(matches!(ghost.click(&target, at(1.0, 1.0), Button::Left, 0), Err(Error::Invalid { .. })));
}

#[test]
fn shot_reports_how_picture_pixels_map_to_window_points() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&1600u32.to_be_bytes());
    png.extend_from_slice(&1200u32.to_be_bytes());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("shot.png");
    let ghost = Ghost::new(Fake { picture: png, ..Fake::default() });
    let shot = ghost.shot(&window(4, "main", 800.0, 600.0, true), &out).unwrap();
    assert_eq!((shot.pixel_width, shot.pixel_height), (1600, 1200));
    assert_eq!(shot.scale, 2.0);
    assert_eq!(shot.to_point(908.0, 988.0), at(454.0, 494.0));
}

#[test]
fn shot_fails_when_the_file_is_not_a_picture() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("shot.png");
    let ghost = Ghost::new(Fake { picture: b"nope".to_vec(), ..Fake::default() });
    assert!(matches!(ghost.shot(&window(4, "main", 800.0, 600.0, true), &out), Err(Error::Capture(_))));
}

fn opening(fake: Fake) -> Ghost<Fake> {
    Ghost::new(Fake { windows: vec![window(1, "x", 10.0, 10.0, true)], front: Cell::new(Some(OWNER_APP)), ..fake })
}

#[test]
fn open_leaves_the_front_alone_when_the_program_stays_behind() {
    let ghost = opening(Fake::default());
    assert_eq!(ghost.open(&Launch::default(), OPEN_TIMEOUT).unwrap().id, 1);
    assert!(ghost.backend().activated.borrow().is_empty());
}

#[test]
fn open_gives_the_front_back_when_the_program_grabs_it_on_start() {
    let ghost = opening(Fake { grabs_after_waits: Some(0), ..Fake::default() });
    ghost.open(&Launch::default(), OPEN_TIMEOUT).unwrap();
    assert_eq!(*ghost.backend().activated.borrow(), vec![OWNER_APP]);
    assert_eq!(ghost.backend().front.get(), Some(OWNER_APP));
}

#[test]
fn open_gives_the_front_back_when_the_program_grabs_it_after_the_window_shows() {
    let ghost = opening(Fake { grabs_after_waits: Some(10), ..Fake::default() });
    ghost.open(&Launch::default(), OPEN_TIMEOUT).unwrap();
    assert_eq!(*ghost.backend().activated.borrow(), vec![OWNER_APP]);
}

#[test]
fn open_never_undoes_a_person_moving_to_another_app() {
    let ghost = opening(Fake { person_moves_to: Some(200), ..Fake::default() });
    ghost.open(&Launch::default(), OPEN_TIMEOUT).unwrap();
    assert!(ghost.backend().activated.borrow().is_empty());
    assert_eq!(ghost.backend().front.get(), Some(200));
}

#[test]
fn open_says_so_when_the_front_cannot_be_given_back() {
    let ghost = opening(Fake { grabs_after_waits: Some(0), activate_fails: true, ..Fake::default() });
    let error = ghost.open(&Launch::default(), OPEN_TIMEOUT).unwrap_err();
    assert!(matches!(error, Error::FrontLost { pid: PROGRAM, front: OWNER_APP }), "{error}");
}

#[test]
fn open_still_reports_a_program_that_shows_no_window() {
    let ghost = Ghost::new(Fake { grabs_after_waits: Some(0), front: Cell::new(Some(OWNER_APP)), ..Fake::default() });
    assert!(matches!(ghost.open(&Launch::default(), OPEN_TIMEOUT), Err(Error::NoWindow { pid: PROGRAM })));
    assert_eq!(ghost.backend().front.get(), Some(OWNER_APP));
}
