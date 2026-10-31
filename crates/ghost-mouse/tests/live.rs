//! Proves on a real window that every gesture lands while the cursor and the front app stay out of it.
//! Opens a window on the desktop, so it only runs with GHOST_MOUSE_LIVE=1.

#![cfg(target_os = "macos")]

use std::{
    fs,
    path::Path,
    process::Command,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use ghost_mouse::{cursor_position, system, Button, Launch, Point, Window};

static WINDOW_TURN: Mutex<()> = Mutex::new(());

/// One window at a time, since one of these tests takes the front on purpose.
fn window_turn() -> MutexGuard<'static, ()> {
    WINDOW_TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct KillOnDrop(i32);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        // intentional: the window quits by itself anyway if this fails
        let _ = Command::new("/bin/kill").arg(self.0.to_string()).status();
    }
}

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn wait_for_line(log: &Path, wanted: &str) {
    let started = Instant::now();
    loop {
        let text = fs::read_to_string(log).unwrap_or_default();
        if text.lines().any(|line| line.starts_with(wanted)) {
            return;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "never saw `{wanted}` in:\n{text}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn front_app() -> String {
    let script = r#"tell application "System Events" to get name of first process whose frontmost is true"#;
    let output = Command::new("/usr/bin/osascript").args(["-e", script]).output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The owner may be using the machine, so the cursor may move; it just must not go where we click.
fn assert_cursor_is_not_on(window: &Window, point: Point) {
    let cursor = cursor_position().unwrap();
    let (x, y) = (window.x + point.x, window.y + point.y);
    assert!((cursor.x - x).abs() > 1.0 || (cursor.y - y).abs() > 1.0, "the real cursor went to {x},{y}");
}

#[test]
fn every_gesture_lands_without_touching_cursor_or_front_app() {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return;
    }
    let _turn = window_turn();
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let binary = dir.path().join("ghost-target");
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/target.swift");
    let built = Command::new("/usr/bin/swiftc").arg(source).arg("-o").arg(&binary).status().unwrap();
    assert!(built.success());
    let log = dir.path().join("events.log");

    let ghost = system();
    let launch = Launch { program: binary, args: vec![log.to_string_lossy().into_owned()], env: vec![] };
    let window = ghost.open(&launch, Duration::from_secs(10)).unwrap();
    let _kill = KillOnDrop(window.pid);

    ghost.click(&window, at(180.0, 112.0), Button::Left, 1).unwrap();
    wait_for_line(&log, "down left x=180 y=112 clicks=1");
    wait_for_line(&log, "up left x=180 y=112");
    assert_cursor_is_not_on(&window, at(180.0, 112.0));

    ghost.click(&window, at(30.0, 40.0), Button::Left, 2).unwrap();
    wait_for_line(&log, "down left x=30 y=40 clicks=2");

    ghost.click(&window, at(200.0, 150.0), Button::Right, 1).unwrap();
    wait_for_line(&log, "down right x=200 y=150 clicks=1");

    ghost.click(&window, at(210.0, 160.0), Button::Middle, 1).unwrap();
    wait_for_line(&log, "down middle x=210 y=160 button=2");

    ghost.drag(&window, at(50.0, 60.0), at(250.0, 180.0), Button::Left, 4).unwrap();
    wait_for_line(&log, "drag x=150 y=120");
    wait_for_line(&log, "up left x=250 y=180");
    assert_cursor_is_not_on(&window, at(250.0, 180.0));

    ghost.hover(&window, at(120.0, 90.0)).unwrap();
    wait_for_line(&log, "moved x=120 y=90");

    ghost.scroll(&window, at(100.0, 100.0), 0, 3).unwrap();
    wait_for_line(&log, "scroll dx=0 dy=-3");

    ghost.press(&window, "x".parse().unwrap()).unwrap();
    wait_for_line(&log, "key chars=x code=7");
    ghost.press(&window, "cmd+shift+a".parse().unwrap()).unwrap();
    wait_for_line(&log, "key chars=a code=0 cmd=true shift=true");
    ghost.type_text(&window, "açã").unwrap();
    wait_for_line(&log, "key chars=ç");
    wait_for_line(&log, "key chars=ã");

    let picture = dir.path().join("shot.png");
    let shot = ghost.shot(&window, &picture).unwrap();
    assert!(shot.scale >= 1.0, "{shot:?}");
    assert!((f64::from(shot.pixel_height) - window.height * shot.scale).abs() <= 2.0 * shot.scale, "{shot:?}");

    assert_ne!(front_app(), window.owner);
    let events = fs::read_to_string(&log).unwrap();
    assert!(!events.contains("became-active"), "the target took the focus:\n{events}");
}

#[test]
fn a_program_that_grabs_the_front_gives_it_back() {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return;
    }
    let _turn = window_turn();
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let binary = dir.path().join("ghost-target");
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/target.swift");
    let built = Command::new("/usr/bin/swiftc").arg(source).arg("-o").arg(&binary).status().unwrap();
    assert!(built.success());
    let log = dir.path().join("events.log");

    let ghost = system();
    let launch = Launch {
        program: binary,
        args: vec![log.to_string_lossy().into_owned()],
        env: vec![("GHOST_TARGET_GRAB".into(), "1".into())],
    };
    let window = ghost.open(&launch, Duration::from_secs(10)).unwrap();
    let _kill = KillOnDrop(window.pid);

    wait_for_line(&log, "became-active");
    assert_ne!(front_app(), window.owner);
}
