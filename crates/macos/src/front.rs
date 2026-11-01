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

/// Each app the system starts gets the next serial number. Once the count passes 32 bits, the
/// programs it starts stay stopped and `CGEvent.postToPid` silently drops their events, until a restart.
pub(crate) fn check_delivery(pid: i32) -> Result<()> {
    match ask(&["info", "-only", "ASN", &pid.to_string()]) {
        Some(answer) if past_32_bits(&answer) => Err(Error::EventsLost { pid }),
        _ => Ok(()),
    }
}

/// Reads the high half of a serial number such as `"LSASN"=ASN:0x1-0xed4ac3c:`.
fn past_32_bits(answer: &str) -> bool {
    answer
        .split_once("ASN:0x")
        .and_then(|(_, rest)| rest.split_once('-'))
        .is_some_and(|(high, _)| u32::from_str_radix(high, 16).is_ok_and(|high| high != 0))
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
    use super::{parse_pid, past_32_bits};

    #[test]
    fn a_serial_number_past_32_bits_cannot_receive_events() {
        assert!(past_32_bits("\"LSASN\"=ASN:0x1-0xed4ac3c:(pid=248818748)\n"));
        assert!(!past_32_bits("\"LSASN\"=ASN:0x0-0xfe58c48e:\n"));
    }

    #[test]
    fn a_process_with_no_serial_number_is_left_alone() {
        assert!(!past_32_bits(""));
        assert!(!past_32_bits("\"LSASN\"=[ NULL ]\n"));
    }

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
