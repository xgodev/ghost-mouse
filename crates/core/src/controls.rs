//! Finds a window's controls by what the program calls them and acts on the one found, never on a guess.

use std::{fmt, time::Duration};

use serde::Serialize;

use crate::{
    backend::Backend,
    error::{Error, Result},
    geometry::{Frame, Window},
    ghost::Ghost,
    input::Button,
};

const CONTROL_POLL: Duration = Duration::from_millis(200);
/// How many controls an error lists before saying how many more there are.
const LISTED: usize = 30;

/// One element of a window's interface, as the program describes it to assistive tools.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Control {
    /// The role as the system names it, such as `AXButton` on macOS.
    pub role: String,
    /// Its title, or its description when it has no title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    pub frame: Frame,
    /// Its centre shows inside the window and inside every scrolling area that holds it.
    pub visible: bool,
}

impl fmt::Display for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.role)?;
        if let Some(name) = &self.name {
            write!(f, " {name:?}")?;
        }
        let Frame { x, y, width, height } = self.frame;
        write!(f, " at {x},{y} ({width}x{height})")
    }
}

/// What a control must be called to match; `None` matches anything. Both compare whole and exact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub name: Option<String>,
    pub role: Option<String>,
}

impl Query {
    /// Whether the control has this name and this role.
    pub fn matches(&self, control: &Control) -> bool {
        let same = |wanted: &Option<String>, actual: Option<&str>| wanted.as_deref().is_none_or(|w| Some(w) == actual);
        same(&self.name, control.name.as_deref()) && same(&self.role, Some(&control.role))
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.name, &self.role) {
            (Some(name), Some(role)) => write!(f, "{role} {name:?}"),
            (Some(name), None) => write!(f, "{name:?}"),
            (None, Some(role)) => write!(f, "{role}"),
            (None, None) => write!(f, "of any kind"),
        }
    }
}

/// Each control on its own line, cut after [`LISTED`] with how many were left out.
pub(crate) fn lines(controls: &[String]) -> String {
    let mut text: String = controls.iter().take(LISTED).map(|c| format!("\n  {c}")).collect();
    if controls.len() > LISTED {
        text.push_str(&format!("\n  and {} more", controls.len() - LISTED));
    }
    text
}

impl<B: Backend> Ghost<B> {
    /// Lists the window's controls that match, in the order the program lists them, waiting up to `wait`
    /// for at least one. An empty list is not an error.
    pub fn controls(&self, window: &Window, query: &Query, wait: Duration) -> Result<Vec<Control>> {
        Ok(self.matching(window, query, wait)?.1)
    }

    /// Finds the one control that matches; `nth` picks among several, counting from 1.
    ///
    /// No match, several without `nth`, or one the window does not show fails with what was found.
    pub fn find_control(&self, window: &Window, query: &Query, nth: Option<usize>, wait: Duration) -> Result<Control> {
        if nth == Some(0) {
            return Err(Error::Invalid { what: "nth", value: "0".into() });
        }
        let (all, mut found) = self.matching(window, query, wait)?;
        let (id, wanted) = (window.id, query.to_string());
        let control = match (found.len(), nth) {
            (0, _) => {
                let candidates = all.iter().filter(|c| c.name.is_some()).map(ToString::to_string).collect();
                return Err(Error::NoControl { window: id, query: wanted, candidates });
            }
            (1, None) => found.remove(0),
            (_, None) => {
                let found = found.iter().map(ToString::to_string).collect();
                return Err(Error::ManyControls { window: id, query: wanted, found });
            }
            (count, Some(nth)) if nth > count => return Err(Error::NoNth { window: id, query: wanted, nth, count }),
            (_, Some(nth)) => found.remove(nth - 1),
        };
        let centre = control.frame.center();
        let shown = (0.0..window.width).contains(&centre.x) && (0.0..window.height).contains(&centre.y);
        if !control.visible || !shown {
            return Err(Error::ControlHidden { window: id, control: control.to_string() });
        }
        Ok(control)
    }

    /// Clicks the centre of the one control that matches, the way a click at a point goes, and returns it.
    pub fn click_control(
        &self,
        window: &Window,
        query: &Query,
        nth: Option<usize>,
        button: Button,
        clicks: u8,
        wait: Duration,
    ) -> Result<Control> {
        let control = self.find_control(window, query, nth, wait)?;
        self.click(window, control.frame.center(), button, clicks)?;
        Ok(control)
    }

    /// Rests the pointer on the centre of the one control that matches and returns it.
    pub fn hover_control(&self, window: &Window, query: &Query, nth: Option<usize>, wait: Duration) -> Result<Control> {
        let control = self.find_control(window, query, nth, wait)?;
        self.hover(window, control.frame.center())?;
        Ok(control)
    }

    /// Every control of the window and the ones that match, once at least one does or `wait` is over.
    fn matching(&self, window: &Window, query: &Query, wait: Duration) -> Result<(Vec<Control>, Vec<Control>)> {
        let mut waited = Duration::ZERO;
        loop {
            let all = self.backend().controls(window)?;
            let found: Vec<_> = all.iter().filter(|c| query.matches(c)).cloned().collect();
            if !found.is_empty() || waited >= wait {
                return Ok((all, found));
            }
            self.backend().wait(CONTROL_POLL);
            waited += CONTROL_POLL;
        }
    }
}
