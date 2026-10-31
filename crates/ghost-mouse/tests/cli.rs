//! The command line refuses bad input with a clear message, before touching any program.

use std::process::{Command, Output};

fn ghost(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ghost-mouse")).args(args).output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn bad_point_is_refused() {
    let output = ghost(&["click", "--pid", "1", "--at", "nope"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("invalid point"), "{}", stderr(&output));
}

#[test]
fn a_process_without_windows_is_reported() {
    let output = ghost(&["click", "--pid", "999999", "--at", "1,1"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("process 999999 has no visible window"), "{}", stderr(&output));
}

#[test]
fn unknown_key_is_refused() {
    let output = ghost(&["key", "--pid", "1", "hyper+a"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("invalid key"), "{}", stderr(&output));
}

#[test]
fn listing_prints_json() {
    let output = ghost(&["windows", "--pid", "999999"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
}
