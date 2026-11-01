//! Proves on a real window that every gesture lands while the cursor and the front app stay out of it.
//! Opens a window on the desktop, so it only runs with GHOST_MOUSE_LIVE=1.

#![cfg(target_os = "macos")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use ghost_mouse::{cursor_position, system, Backend, Button, Error, Launch, Point, Window};

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
fn a_bare_program_receives_the_library_path_it_was_given() {
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
        env: vec![("DYLD_FALLBACK_LIBRARY_PATH".into(), "/a b/lib".into())],
    };
    let window = ghost.open(&launch, Duration::from_secs(10)).unwrap();
    let _kill = KillOnDrop(window.pid);

    wait_for_line(&log, "library-path=/a b/lib");
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

fn floating_list(pid: i32) -> Window {
    let started = Instant::now();
    loop {
        if let Some(list) = system().windows(Some(pid)).unwrap().into_iter().find(|w| w.layer != 0) {
            return list;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "the program's list never showed in the window list");
        thread::sleep(Duration::from_millis(20));
    }
}

fn open_with_list(mode: &str) -> (tempfile::TempDir, PathBuf, Window) {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let binary = dir.path().join("ghost-target");
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/target.swift");
    let built = Command::new("/usr/bin/swiftc").arg(source).arg("-o").arg(&binary).status().unwrap();
    assert!(built.success());
    let log = dir.path().join("events.log");
    let launch = Launch {
        program: binary,
        args: vec![log.to_string_lossy().into_owned()],
        env: vec![("GHOST_TARGET_LIST".into(), mode.into())],
    };
    let window = system().open(&launch, Duration::from_secs(10)).unwrap();
    (dir, log, window)
}

#[test]
fn a_list_macos_keeps_off_the_screen_can_be_seen_and_clicked() {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return;
    }
    let _turn = window_turn();
    let ghost = system();
    let (dir, log, window) = open_with_list("hidden");
    let _kill = KillOnDrop(window.pid);

    ghost.click(&window, at(180.0, 112.0), Button::Left, 1).unwrap();
    wait_for_line(&log, "list open");
    let list = floating_list(window.pid);
    assert!(list.held_back && !list.on_screen, "{list:?}");
    assert_eq!(ghost.main_window(window.pid).unwrap().id, window.id);

    let shot = ghost.shot(&list, &dir.path().join("list.png")).unwrap();
    assert!(shot.pixel_width > 0);

    ghost.click(&list, at(20.0, 30.0), Button::Left, 1).unwrap();
    wait_for_line(&log, "list down x=20 y=30");
    assert_cursor_is_not_on(&list, at(20.0, 30.0));

    let started = Instant::now();
    while ghost.windows(Some(window.pid)).unwrap().iter().any(|w| w.id == list.id) {
        assert!(started.elapsed() < Duration::from_secs(5), "a closed list stayed in the window list");
        thread::sleep(Duration::from_millis(20));
    }
    assert_ne!(front_app(), window.owner);
    let events = fs::read_to_string(&log).unwrap();
    assert!(!events.contains("became-active"), "the target took the focus:\n{events}");
}

#[test]
fn a_list_on_the_screen_is_listed_but_never_taken_for_the_main_window() {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return;
    }
    let _turn = window_turn();
    let ghost = system();
    let (_dir, log, window) = open_with_list("stays");
    let _kill = KillOnDrop(window.pid);

    ghost.click(&window, at(180.0, 112.0), Button::Left, 1).unwrap();
    wait_for_line(&log, "list open");
    let list = floating_list(window.pid);
    assert!(list.on_screen && !list.held_back, "{list:?}");
    assert_eq!(ghost.main_window(window.pid).unwrap().id, window.id);
}

#[test]
fn a_button_in_a_web_page_is_clicked_behind_the_others() {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return;
    }
    let _turn = window_turn();
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let binary = dir.path().join("ghost-web");
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/web.swift");
    let built = Command::new("/usr/bin/swiftc").arg(source).arg("-o").arg(&binary).status().unwrap();
    assert!(built.success());
    let log = dir.path().join("events.log");

    let ghost = system();
    let launch = Launch { program: binary, args: vec![log.to_string_lossy().into_owned()], env: vec![] };
    let window = ghost.open(&launch, Duration::from_secs(10)).unwrap();
    let _kill = KillOnDrop(window.pid);
    wait_for_line(&log, "ready");

    let button = at(180.0, 138.0);
    ghost.click(&window, button, Button::Left, 1).unwrap();
    wait_for_line(&log, "page click");
    assert_cursor_is_not_on(&window, button);

    assert_ne!(front_app(), window.owner);
    let events = fs::read_to_string(&log).unwrap();
    assert!(!events.contains("became-active"), "the target took the focus:\n{events}");
}

/// A program behind the others, showing a native file panel on a small folder tree:
/// `a/deep/file.txt`, `a/açã.txt`, `b/here.txt`, `b/existing.txt`, `c/` and `.hidden/secret.txt`.
struct PanelProgram {
    _kill: KillOnDrop,
    dir: tempfile::TempDir,
    log: PathBuf,
    pid: i32,
    /// The panel's window; the cursor has no business going there.
    panel: Window,
    front: Option<i32>,
}

impl PanelProgram {
    fn open(shows: &str, env: &[&str]) -> Self {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let root = dir.path().canonicalize().unwrap();
        for folder in ["a/deep", "b", "c", ".hidden"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        for file in ["a/deep/file.txt", "a/açã.txt", "b/here.txt", "b/existing.txt", ".hidden/secret.txt"] {
            fs::write(root.join(file), "").unwrap();
        }
        let binary = dir.path().join("ghost-panel");
        let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/panel.swift");
        let built = Command::new("/usr/bin/swiftc").arg(source).arg("-o").arg(&binary).status().unwrap();
        assert!(built.success());
        let log = dir.path().join("panel.log");

        let ghost = system();
        let launch = Launch {
            program: binary,
            args: vec![log.to_string_lossy().into_owned(), root.join(shows).to_string_lossy().into_owned()],
            env: env.iter().map(|name| (name.to_string(), "1".to_string())).collect(),
        };
        let front = ghost.backend().front_pid();
        let pid = ghost.open(&launch, Duration::from_secs(10)).unwrap().pid;
        wait_for_line(&log, "panel-opening");
        let started = Instant::now();
        while ghost.backend().panel(pid).unwrap().is_none() {
            assert!(started.elapsed() < Duration::from_secs(5), "the panel never showed");
            thread::sleep(Duration::from_millis(50));
        }
        let panel = ghost.main_window(pid).unwrap();
        Self { _kill: KillOnDrop(pid), dir, log, pid, panel, front }
    }

    fn path(&self, part: &str) -> PathBuf {
        self.dir.path().canonicalize().unwrap().join(part)
    }

    /// The owner may be using the machine, so both may change; neither may go to this program.
    fn assert_cursor_and_front_stayed_out(&self, cursor_at_start: Point) {
        let cursor = cursor_position().unwrap();
        let on_panel = |at: Point| {
            (self.panel.x..self.panel.x + self.panel.width).contains(&at.x)
                && (self.panel.y..self.panel.y + self.panel.height).contains(&at.y)
        };
        assert!(!on_panel(cursor) || on_panel(cursor_at_start), "the real cursor went to the panel: {cursor:?}");
        assert_ne!(system().backend().front_pid(), Some(self.pid), "the program stayed in front");
        assert_ne!(self.front, Some(self.pid), "the program was in front before the test");
    }

    fn assert_chose(&self, wanted: &Path) {
        wait_for_line(&self.log, &format!("chose {}", wanted.display()));
    }
}

const PANEL_WAIT: Duration = Duration::from_secs(40);

fn live_panel() -> Option<MutexGuard<'static, ()>> {
    if std::env::var_os("GHOST_MOUSE_LIVE").is_none() {
        eprintln!("SKIPPED: set GHOST_MOUSE_LIVE=1 to open a real window");
        return None;
    }
    Some(window_turn())
}

#[test]
fn a_file_is_chosen_in_a_panel_behind_the_others() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &[]);
    let cursor = cursor_position().unwrap();
    let wanted = program.path("b/here.txt");
    assert_eq!(system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap(), wanted);
    program.assert_chose(&wanted);
    program.assert_cursor_and_front_stayed_out(cursor);
}

#[test]
fn a_file_elsewhere_is_reached_through_the_panel() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("c", &[]);
    let cursor = cursor_position().unwrap();
    let wanted = program.path("a/deep/file.txt");
    system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap();
    program.assert_chose(&wanted);
    program.assert_cursor_and_front_stayed_out(cursor);
}

#[test]
fn an_accented_name_is_chosen() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("a", &[]);
    let wanted = program.path("a/açã.txt");
    system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap();
    program.assert_chose(&wanted);
}

#[test]
fn a_name_stored_with_decomposed_accents_is_chosen() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("a", &[]);
    fs::write(program.path("a/cafe\u{301}.txt"), "").unwrap();
    system().choose_file(program.pid, &program.path("a/caf\u{e9}.txt"), PANEL_WAIT).unwrap();
    program.assert_chose(&program.path("a/caf\u{e9}.txt"));
}

#[test]
fn a_file_the_panel_hides_confirms_nothing() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &[]);
    let hidden = program.path(".hidden/secret.txt");
    let error = system().choose_file(program.pid, &hidden, PANEL_WAIT).unwrap_err();
    assert!(matches!(&error, Error::NotInPanel { path } if *path == hidden), "{error}");
    system().cancel_file_panel(program.pid, PANEL_WAIT).unwrap();
    wait_for_line(&program.log, "cancelled");
}

#[test]
fn a_panel_is_cancelled_behind_the_others() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &[]);
    let cursor = cursor_position().unwrap();
    system().cancel_file_panel(program.pid, PANEL_WAIT).unwrap();
    wait_for_line(&program.log, "cancelled");
    program.assert_cursor_and_front_stayed_out(cursor);
}

#[test]
fn a_new_file_is_named_in_a_save_panel() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &["GHOST_PANEL_SAVE"]);
    let cursor = cursor_position().unwrap();
    let wanted = program.path("b/novo açã.txt");
    assert_eq!(system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap(), wanted);
    program.assert_chose(&wanted);
    program.assert_cursor_and_front_stayed_out(cursor);
}

#[test]
fn a_new_file_in_another_folder_is_saved_there() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &["GHOST_PANEL_SAVE"]);
    let wanted = program.path("a/deep/novo.txt");
    system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap();
    program.assert_chose(&wanted);
}

#[test]
fn a_new_file_in_an_empty_folder_is_saved_there() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("c", &["GHOST_PANEL_SAVE"]);
    let wanted = program.path("c/novo.txt");
    system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap();
    program.assert_chose(&wanted);
}

#[test]
fn a_save_panel_never_replaces_a_file() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &["GHOST_PANEL_SAVE"]);
    let error = system().choose_file(program.pid, &program.path("b/existing.txt"), PANEL_WAIT).unwrap_err();
    assert!(matches!(error, Error::Invalid { .. }), "{error}");
    system().cancel_file_panel(program.pid, PANEL_WAIT).unwrap();
}

#[test]
fn a_panel_shown_as_a_sheet_is_reached() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &["GHOST_PANEL_SHEET"]);
    let wanted = program.path("b/here.txt");
    system().choose_file(program.pid, &wanted, PANEL_WAIT).unwrap();
    program.assert_chose(&wanted);
}

#[test]
fn a_file_the_panel_filters_out_is_refused() {
    let Some(_turn) = live_panel() else { return };
    let program = PanelProgram::open("b", &["GHOST_PANEL_ONLY_PNG"]);
    let error = system().choose_file(program.pid, &program.path("b/here.txt"), PANEL_WAIT).unwrap_err();
    assert!(matches!(error, Error::PanelRefused), "{error}");
    system().cancel_file_panel(program.pid, PANEL_WAIT).unwrap();
}
