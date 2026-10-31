//! Reads and restores which app is in front.

use std::process::Command;

use ghost_mouse_core::{Error, Result};
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

const LSAPPINFO: &str = "/usr/bin/lsappinfo";

/// Asks the system on every call, because a process with no event loop only sees a stale answer from AppKit.
pub(crate) fn front_pid() -> Option<i32> {
    let front = ask(&["front"])?;
    parse_pid(&ask(&["info", "-only", "pid", "-app", front.trim()])?)
}

pub(crate) fn activate(pid: i32) -> Result<()> {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .ok_or_else(|| Error::Event(format!("process {pid} is not an app")))?;
    if app.activateWithOptions(NSApplicationActivationOptions::empty()) {
        return Ok(());
    }
    Err(Error::Event(format!("process {pid} refused to come to the front")))
}

fn ask(args: &[&str]) -> Option<String> {
    let output = Command::new(LSAPPINFO).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_pid(answer: &str) -> Option<i32> {
    answer.trim().strip_prefix("\"pid\"=")?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_pid;

    #[test]
    fn reads_the_process_id_from_the_answer() {
        assert_eq!(parse_pid("\"pid\"=4141\n"), Some(4141));
    }

    #[test]
    fn an_answer_with_no_process_is_no_process() {
        assert_eq!(parse_pid("\"pid\"=[ NULL ]\n"), None);
        assert_eq!(parse_pid(""), None);
    }
}
