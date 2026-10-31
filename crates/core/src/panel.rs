//! Completes a native file panel by walking it to a path through the rows it lists.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    backend::Backend,
    error::{Error, Result},
    ghost::Ghost,
};

const PANEL_POLL: Duration = Duration::from_millis(200);
/// A new list that stops changing for this long has arrived. Reading one can take seconds, so this stays short.
const LIST_SETTLE: Duration = Duration::from_millis(400);
/// An empty list, or the one from before going up, may still be on its way.
const LIST_ARRIVING: Duration = Duration::from_millis(1400);
/// How long a panel gets to answer a press.
const PANEL_ANSWER: Duration = Duration::from_secs(5);

/// What a file panel is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Open,
    Save,
}

/// A file panel as it stands right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub kind: PanelKind,
    /// The real path of every row listed, in any folder expanded on the way.
    pub rows: Vec<PathBuf>,
}

impl<B: Backend> Ghost<B> {
    /// Completes the file panel of a process with `path` and returns the path chosen.
    ///
    /// Nothing is confirmed unless the panel lists the path. A save panel never replaces a file.
    pub fn choose_file(&self, pid: i32, path: &Path, timeout: Duration) -> Result<PathBuf> {
        let front = self.backend().front_pid();
        let panel = self.wait_for_panel(pid, timeout)?;
        let chosen = match panel.kind {
            PanelKind::Open => self.open_file(pid, path, timeout)?,
            PanelKind::Save => self.save_file(pid, path, timeout)?,
        };
        if !self.panel_closes(pid)? {
            return Err(Error::PanelStayedOpen { path: chosen });
        }
        self.give_front_back(pid, front)?;
        Ok(chosen)
    }

    /// Cancels the file panel of a process.
    pub fn cancel_file_panel(&self, pid: i32, timeout: Duration) -> Result<()> {
        let front = self.backend().front_pid();
        self.wait_for_panel(pid, timeout)?;
        self.backend().panel_cancel(pid)?;
        if !self.panel_closes(pid)? {
            return Err(Error::PanelNotCancelled { pid });
        }
        self.give_front_back(pid, front)
    }

    fn give_front_back(&self, pid: i32, front: Option<i32>) -> Result<()> {
        match front.filter(|front| *front != pid) {
            Some(front) => self.keep_front(pid, front),
            None => Ok(()),
        }
    }

    fn wait_for_panel(&self, pid: i32, timeout: Duration) -> Result<Panel> {
        let mut waited = Duration::ZERO;
        loop {
            if let Some(panel) = self.backend().panel(pid)? {
                return Ok(panel);
            }
            if waited >= timeout {
                return Err(Error::NoFilePanel { pid });
            }
            self.backend().wait(PANEL_POLL);
            waited += PANEL_POLL;
        }
    }

    fn panel_closes(&self, pid: i32) -> Result<bool> {
        let mut waited = Duration::ZERO;
        while self.backend().panel(pid)?.is_some() {
            if waited >= PANEL_ANSWER {
                return Ok(false);
            }
            self.backend().wait(PANEL_POLL);
            waited += PANEL_POLL;
        }
        Ok(true)
    }

    fn open_file(&self, pid: i32, path: &Path, timeout: Duration) -> Result<PathBuf> {
        let wanted = path.canonicalize()?;
        self.walk(pid, &wanted, timeout, |rows| rows.contains(&wanted))?;
        self.backend().panel_select(pid, &wanted)?;
        self.backend().panel_confirm(pid)?;
        Ok(wanted)
    }

    fn save_file(&self, pid: i32, path: &Path, timeout: Duration) -> Result<PathBuf> {
        let invalid = || Error::Invalid { what: "path to save", value: path.display().to_string() };
        let name = path.file_name().and_then(|name| name.to_str()).ok_or_else(invalid)?;
        let absolute = std::path::absolute(path)?;
        let folder = absolute.parent().ok_or_else(invalid)?.canonicalize()?;
        let wanted = folder.join(name);
        if wanted.exists() {
            return Err(Error::Invalid {
                what: "path to save, it already exists",
                value: wanted.display().to_string(),
            });
        }
        let shows = |rows: &[PathBuf]| shown_folder(rows).as_deref() == Some(folder.as_path());
        let rows = self.walk(pid, &folder, timeout, |rows| rows.contains(&folder) || shows(rows))?;
        if !shows(&rows) {
            self.enter(pid, &folder)?;
        }
        self.backend().panel_name(pid, name)?;
        self.backend().panel_confirm(pid)?;
        Ok(wanted)
    }

    /// Confirming on a folder listed goes into it. Rows still arriving from before must not pass
    /// for that, or the name would land in the wrong folder.
    fn enter(&self, pid: i32, folder: &Path) -> Result<()> {
        self.backend().panel_select(pid, folder)?;
        self.backend().panel_confirm(pid)?;
        let mut waited = Duration::ZERO;
        loop {
            let rows = self.listed(pid)?;
            if rows.is_empty() || shown_folder(&rows).as_deref() == Some(folder) {
                return Ok(());
            }
            if waited >= PANEL_ANSWER {
                return Err(Error::NotInPanel { path: folder.to_path_buf() });
            }
            self.backend().wait(PANEL_POLL);
            waited += PANEL_POLL;
        }
    }

    fn listed(&self, pid: i32) -> Result<Vec<PathBuf>> {
        Ok(self.backend().panel(pid)?.ok_or(Error::NoFilePanel { pid })?.rows)
    }

    /// Expands the folders on the way to `goal`, going up when the list holds none of them,
    /// until `arrived` accepts what is listed.
    fn walk(
        &self,
        pid: i32,
        goal: &Path,
        timeout: Duration,
        arrived: impl Fn(&[PathBuf]) -> bool,
    ) -> Result<Vec<PathBuf>> {
        let not_listed = || Error::NotInPanel { path: goal.to_path_buf() };
        let started = Instant::now();
        let mut expanded = HashSet::new();
        let mut last: Option<Vec<PathBuf>> = None;
        let mut before_up: Option<Vec<PathBuf>> = None;
        let mut quiet = Duration::ZERO;
        let mut waited = Duration::ZERO;
        loop {
            let rows = self.listed(pid)?;
            if arrived(&rows) {
                return Ok(rows);
            }
            let arriving = rows.is_empty() || before_up.as_ref() == Some(&rows);
            if let Some(row) = deepest_on_the_way(&rows, goal, &expanded) {
                self.backend().panel_expand(pid, &row)?;
                expanded.insert(row);
                quiet = Duration::ZERO;
            } else if quiet >= if arriving { LIST_ARRIVING } else { LIST_SETTLE } {
                if !self.backend().panel_up(pid)? {
                    return Err(not_listed());
                }
                before_up = Some(rows.clone());
                quiet = Duration::ZERO;
            }
            // Reading a long list takes real time, on top of the waits.
            if waited.max(started.elapsed()) >= timeout {
                return Err(not_listed());
            }
            self.backend().wait(PANEL_POLL);
            waited += PANEL_POLL;
            quiet += PANEL_POLL;
            if last.as_ref() != Some(&rows) {
                quiet = Duration::ZERO;
                last = Some(rows);
            }
        }
    }
}

fn deepest_on_the_way(rows: &[PathBuf], goal: &Path, expanded: &HashSet<PathBuf>) -> Option<PathBuf> {
    rows.iter()
        .filter(|row| goal != row.as_path() && goal.starts_with(row) && !expanded.contains(*row))
        .max_by_key(|row| row.components().count())
        .cloned()
}

/// The folder a list shows: rows of expanded folders always sit deeper than its own.
/// The panel's top level lists volumes, and belongs to no folder.
fn shown_folder(rows: &[PathBuf]) -> Option<PathBuf> {
    let parents: Option<Vec<&Path>> = rows.iter().map(|row| row.parent()).collect();
    parents?.into_iter().min_by_key(|parent| parent.components().count()).map(Path::to_path_buf)
}
