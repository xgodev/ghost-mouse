//! Completing a file panel walks to the path through what the panel lists, and only then confirms.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use ghost_mouse_core::{Backend, Error, Ghost, Input, Launch, Panel, PanelKind, Result, Window};
use tempfile::TempDir;

const PROGRAM: i32 = 7;
const OWNER_APP: i32 = 100;
const WAIT: Duration = Duration::from_secs(30);

/// A panel over a made-up view of the disk: each folder lists what `folders` says.
struct Fake {
    kind: PanelKind,
    folders: HashMap<PathBuf, Vec<PathBuf>>,
    /// The folder the panel shows.
    current: RefCell<PathBuf>,
    top: PathBuf,
    expanded: RefCell<HashSet<PathBuf>>,
    /// Rows of a folder just opened show after this many more waits.
    list_delay: u32,
    pending: Cell<u32>,
    /// After going up, the old rows stay listed for this many more waits.
    old_rows_stay: u32,
    old_rows: RefCell<Option<(Vec<PathBuf>, u32)>>,
    /// Rows from before keep arriving for this many waits after entering a folder.
    enters_after_waits: u32,
    entering: Cell<u32>,
    /// The panel shows up after this many waits; `None` never.
    shows_after_waits: Option<u32>,
    open: Cell<bool>,
    closes_on_confirm: bool,
    grabs_on_confirm: bool,
    selected: RefCell<Option<PathBuf>>,
    name: RefCell<Option<String>>,
    actions: RefCell<Vec<String>>,
    waits: Cell<u32>,
    front: Cell<Option<i32>>,
    activated: RefCell<Vec<i32>>,
}

impl Fake {
    fn over(top: &Path, current: &Path, kind: PanelKind) -> Self {
        Self {
            kind,
            folders: HashMap::new(),
            current: RefCell::new(current.to_path_buf()),
            top: top.to_path_buf(),
            expanded: RefCell::new(HashSet::new()),
            list_delay: 0,
            pending: Cell::new(0),
            old_rows_stay: 0,
            old_rows: RefCell::new(None),
            enters_after_waits: 0,
            entering: Cell::new(0),
            shows_after_waits: Some(0),
            open: Cell::new(false),
            closes_on_confirm: true,
            grabs_on_confirm: false,
            selected: RefCell::new(None),
            name: RefCell::new(None),
            actions: RefCell::new(vec![]),
            waits: Cell::new(0),
            front: Cell::new(Some(OWNER_APP)),
            activated: RefCell::new(vec![]),
        }
    }

    fn list(&self, folder: &Path, out: &mut Vec<PathBuf>) {
        for row in self.folders.get(folder).into_iter().flatten() {
            out.push(row.clone());
            if self.expanded.borrow().contains(row) {
                self.list(row, out);
            }
        }
    }

    fn shown(&self) -> bool {
        self.open.get() || self.shows_after_waits.is_some_and(|after| self.waits.get() >= after)
    }

    fn did(&self, action: String) {
        self.actions.borrow_mut().push(action);
    }

    fn actions(&self) -> Vec<String> {
        self.actions.borrow().clone()
    }
}

impl Backend for Fake {
    fn windows(&self, _pid: Option<i32>) -> Result<Vec<Window>> {
        Ok(vec![])
    }
    fn send(&self, _window: &Window, _input: &Input) -> Result<()> {
        Ok(())
    }
    fn wait(&self, _time: Duration) {
        self.waits.set(self.waits.get() + 1);
        self.pending.set(self.pending.get().saturating_sub(1));
        self.entering.set(self.entering.get().saturating_sub(1));
        let mut old_rows = self.old_rows.borrow_mut();
        if let Some((_, waits)) = old_rows.as_mut() {
            *waits = waits.saturating_sub(1);
            if *waits == 0 {
                *old_rows = None;
            }
        }
    }
    fn capture(&self, _window: &Window, _out: &Path) -> Result<()> {
        Ok(())
    }
    fn launch(&self, _launch: &Launch) -> Result<i32> {
        Ok(PROGRAM)
    }
    fn front_pid(&self) -> Option<i32> {
        self.front.get()
    }
    fn activate(&self, pid: i32) -> Result<()> {
        self.activated.borrow_mut().push(pid);
        self.front.set(Some(pid));
        Ok(())
    }
    fn panel(&self, _pid: i32) -> Result<Option<Panel>> {
        if !self.shown() || self.closed() {
            return Ok(None);
        }
        self.open.set(true);
        let mut rows = vec![];
        if let Some((old, _)) = self.old_rows.borrow().as_ref() {
            rows.clone_from(old);
        } else if self.entering.get() > 0 {
            self.list(self.current.borrow().parent().unwrap(), &mut rows);
            rows.push(PathBuf::from("/still/arriving"));
            rows.truncate(self.entering.get() as usize);
        } else if self.pending.get() == 0 {
            self.list(&self.current.borrow(), &mut rows);
        }
        Ok(Some(Panel { kind: self.kind, rows }))
    }
    fn panel_up(&self, _pid: i32) -> Result<bool> {
        let current = self.current.borrow().clone();
        if current == self.top {
            return Ok(false);
        }
        if self.old_rows_stay > 0 {
            let mut old = vec![];
            self.list(&current, &mut old);
            *self.old_rows.borrow_mut() = Some((old, self.old_rows_stay));
        }
        *self.current.borrow_mut() = current.parent().unwrap().to_path_buf();
        self.did(format!("up to {}", self.current.borrow().display()));
        self.pending.set(self.list_delay);
        Ok(true)
    }
    fn panel_expand(&self, _pid: i32, row: &Path) -> Result<()> {
        self.expanded.borrow_mut().insert(row.to_path_buf());
        self.did(format!("expand {}", row.display()));
        self.pending.set(self.list_delay);
        Ok(())
    }
    fn panel_select(&self, _pid: i32, row: &Path) -> Result<()> {
        *self.selected.borrow_mut() = Some(row.to_path_buf());
        self.did(format!("select {}", row.display()));
        Ok(())
    }
    fn panel_name(&self, _pid: i32, name: &str) -> Result<()> {
        *self.name.borrow_mut() = Some(name.to_string());
        self.did(format!("name {name}"));
        Ok(())
    }
    fn panel_confirm(&self, _pid: i32) -> Result<()> {
        self.did("confirm".to_string());
        if self.grabs_on_confirm {
            self.front.set(Some(PROGRAM));
        }
        // On a selected folder a save panel enters it instead of closing.
        let selected = self.selected.borrow_mut().take();
        if let Some(folder) = selected.filter(|s| self.kind == PanelKind::Save && s.is_dir()) {
            *self.current.borrow_mut() = folder;
            self.pending.set(self.list_delay);
            self.entering.set(self.enters_after_waits);
            return Ok(());
        }
        if self.closes_on_confirm {
            self.close();
        }
        Ok(())
    }
    fn panel_cancel(&self, _pid: i32) -> Result<()> {
        self.did("cancel".to_string());
        self.close();
        Ok(())
    }
}

impl Fake {
    fn close(&self) {
        self.open.set(false);
        self.did("closed".to_string());
    }
    fn closed(&self) -> bool {
        self.actions.borrow().iter().any(|a| a == "closed")
    }
}

/// A real folder on disk, since paths are checked against it:
/// `root/a/b/file.txt`, `root/a/other.txt`, `root/c/empty/` and `root/.hidden/file.txt`.
struct Disk {
    _dir: TempDir,
    root: PathBuf,
}

impl Disk {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::create_dir_all(root.join("c/empty")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("a/b/file.txt"), "").unwrap();
        fs::write(root.join("a/other.txt"), "").unwrap();
        fs::write(root.join(".hidden/file.txt"), "").unwrap();
        Self { _dir: dir, root }
    }

    /// The panel lists everything but the hidden folder.
    fn fake(&self, current: &str, kind: PanelKind) -> Fake {
        let path = |p: &str| self.root.join(p);
        let mut fake = Fake::over(&self.root, &path(current), kind);
        fake.folders = HashMap::from([
            (self.root.clone(), vec![path("a"), path("c")]),
            (path("a"), vec![path("a/b"), path("a/other.txt")]),
            (path("a/b"), vec![path("a/b/file.txt")]),
            (path("c"), vec![path("c/empty")]),
        ]);
        fake
    }

    fn at(&self, part: &str) -> PathBuf {
        self.root.join(part)
    }
}

#[test]
fn a_file_in_the_folder_shown_is_selected_and_confirmed() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    let chosen = ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT).unwrap();
    assert_eq!(chosen, disk.at("a/other.txt"));
    assert_eq!(
        ghost.backend().actions(),
        vec![format!("select {}", disk.at("a/other.txt").display()), "confirm".into(), "closed".into()]
    );
}

#[test]
fn the_path_is_matched_by_where_it_really_is() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    let chosen = ghost.choose_file(PROGRAM, &disk.at("a/b/../other.txt"), WAIT).unwrap();
    assert_eq!(chosen, disk.at("a/other.txt"));
}

#[test]
fn only_the_folders_on_the_way_are_expanded() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    ghost.choose_file(PROGRAM, &disk.at("a/b/file.txt"), WAIT).unwrap();
    assert_eq!(
        ghost.backend().actions(),
        vec![
            format!("expand {}", disk.at("a/b").display()),
            format!("select {}", disk.at("a/b/file.txt").display()),
            "confirm".into(),
            "closed".into()
        ]
    );
}

#[test]
fn a_file_elsewhere_is_reached_by_going_up_first() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("c/empty", PanelKind::Open));
    ghost.choose_file(PROGRAM, &disk.at("a/b/file.txt"), WAIT).unwrap();
    let actions = ghost.backend().actions();
    assert_eq!(actions[0], format!("up to {}", disk.at("c").display()));
    assert_eq!(actions[1], format!("up to {}", disk.root.display()));
    assert_eq!(*ghost.backend().selected.borrow(), None);
    assert!(actions.contains(&format!("select {}", disk.at("a/b/file.txt").display())));
}

#[test]
fn a_list_that_fills_slowly_is_waited_for() {
    let disk = Disk::new();
    let mut fake = disk.fake("c/empty", PanelKind::Open);
    fake.list_delay = 5;
    let ghost = Ghost::new(fake);
    assert_eq!(ghost.choose_file(PROGRAM, &disk.at("a/b/file.txt"), WAIT).unwrap(), disk.at("a/b/file.txt"));
}

#[test]
fn the_old_list_right_after_going_up_does_not_send_it_further_up() {
    let disk = Disk::new();
    let mut fake = disk.fake("c", PanelKind::Open);
    fake.old_rows_stay = 4;
    let ghost = Ghost::new(fake);
    ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT).unwrap();
    assert_eq!(ghost.backend().actions().iter().filter(|a| a.starts_with("up to")).count(), 1);
}

#[test]
fn a_path_the_panel_never_lists_confirms_nothing() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    let wanted = disk.at(".hidden/file.txt");
    let error = ghost.choose_file(PROGRAM, &wanted, WAIT).unwrap_err();
    assert!(matches!(&error, Error::NotInPanel { path } if *path == wanted), "{error}");
    assert!(!ghost.backend().actions().iter().any(|a| a == "confirm"));
}

#[test]
fn a_file_that_does_not_exist_touches_nothing() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    assert!(matches!(ghost.choose_file(PROGRAM, &disk.at("a/nope.txt"), WAIT), Err(Error::Io(_))));
    assert!(ghost.backend().actions().is_empty());
}

#[test]
fn a_panel_that_shows_late_is_waited_for() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Open);
    fake.shows_after_waits = Some(3);
    let ghost = Ghost::new(fake);
    assert_eq!(ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT).unwrap(), disk.at("a/other.txt"));
}

#[test]
fn no_panel_is_an_error_at_the_deadline() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Open);
    fake.shows_after_waits = None;
    let ghost = Ghost::new(fake);
    let error = ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), Duration::from_secs(1)).unwrap_err();
    assert!(matches!(error, Error::NoFilePanel { pid: PROGRAM }), "{error}");
}

#[test]
fn a_panel_that_stays_open_after_confirming_is_an_error() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Open);
    fake.closes_on_confirm = false;
    let ghost = Ghost::new(fake);
    let wanted = disk.at("a/other.txt");
    let error = ghost.choose_file(PROGRAM, &wanted, WAIT).unwrap_err();
    assert!(matches!(&error, Error::PanelStayedOpen { path } if *path == wanted), "{error}");
}

#[test]
fn saving_in_the_folder_shown_only_names_and_confirms() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Save));
    let chosen = ghost.choose_file(PROGRAM, &disk.at("a/new.txt"), WAIT).unwrap();
    assert_eq!(chosen, disk.at("a/new.txt"));
    assert_eq!(ghost.backend().actions(), vec!["name new.txt".to_string(), "confirm".into(), "closed".into()]);
}

#[test]
fn saving_in_another_folder_enters_it_first() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Save);
    fake.list_delay = 3;
    let ghost = Ghost::new(fake);
    ghost.choose_file(PROGRAM, &disk.at("a/b/new.txt"), WAIT).unwrap();
    assert_eq!(
        ghost.backend().actions(),
        vec![
            format!("select {}", disk.at("a/b").display()),
            "confirm".into(),
            "name new.txt".into(),
            "confirm".into(),
            "closed".into()
        ]
    );
    assert_eq!(*ghost.backend().current.borrow(), disk.at("a/b"));
}

#[test]
fn the_name_waits_for_the_list_of_the_folder_entered() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Save);
    fake.enters_after_waits = 3;
    let ghost = Ghost::new(fake);
    ghost.choose_file(PROGRAM, &disk.at("a/b/new.txt"), WAIT).unwrap();
    assert!(ghost.backend().waits.get() >= 3, "named after {} waits", ghost.backend().waits.get());
}

#[test]
fn saving_into_an_empty_folder_shown_enters_it_from_its_parent() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("c/empty", PanelKind::Save));
    ghost.choose_file(PROGRAM, &disk.at("c/empty/new.txt"), WAIT).unwrap();
    assert_eq!(*ghost.backend().current.borrow(), disk.at("c/empty"));
    assert_eq!(*ghost.backend().name.borrow(), Some("new.txt".to_string()));
}

#[test]
fn saving_over_a_file_is_refused() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Save));
    assert!(matches!(ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT), Err(Error::Invalid { .. })));
    assert!(ghost.backend().actions().is_empty());
}

#[test]
fn saving_in_a_folder_that_does_not_exist_touches_nothing() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Save));
    assert!(matches!(ghost.choose_file(PROGRAM, &disk.at("nope/new.txt"), WAIT), Err(Error::Io(_))));
    assert!(ghost.backend().actions().is_empty());
}

#[test]
fn cancel_presses_cancel() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    ghost.cancel_file_panel(PROGRAM, WAIT).unwrap();
    assert_eq!(ghost.backend().actions(), vec!["cancel".to_string(), "closed".into()]);
}

#[test]
fn cancel_with_no_panel_is_an_error() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Open);
    fake.shows_after_waits = None;
    let ghost = Ghost::new(fake);
    let error = ghost.cancel_file_panel(PROGRAM, Duration::from_secs(1)).unwrap_err();
    assert!(matches!(error, Error::NoFilePanel { pid: PROGRAM }), "{error}");
}

#[test]
fn the_front_goes_back_when_the_program_takes_it_on_confirm() {
    let disk = Disk::new();
    let mut fake = disk.fake("a", PanelKind::Open);
    fake.grabs_on_confirm = true;
    let ghost = Ghost::new(fake);
    ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT).unwrap();
    assert_eq!(*ghost.backend().activated.borrow(), vec![OWNER_APP]);
    assert_eq!(ghost.backend().front.get(), Some(OWNER_APP));
}

#[test]
fn the_front_is_left_alone_when_the_program_stays_behind() {
    let disk = Disk::new();
    let ghost = Ghost::new(disk.fake("a", PanelKind::Open));
    ghost.choose_file(PROGRAM, &disk.at("a/other.txt"), WAIT).unwrap();
    assert!(ghost.backend().activated.borrow().is_empty());
}
