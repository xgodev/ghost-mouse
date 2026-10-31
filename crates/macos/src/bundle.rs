//! Opens programs in the background; a bare executable goes through a throwaway app wrapper.

use std::{
    collections::HashSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{self, Command},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ghost_mouse_core::{Error, Launch, Result};

const LAUNCH_DEADLINE: Duration = Duration::from_secs(20);
const LAUNCH_POLL: Duration = Duration::from_millis(50);

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>launcher</string>
<key>CFBundleIdentifier</key><string>dev.xgodev.ghost-mouse.target</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSUIElement</key><true/>
</dict></plist>
"#;

/// Opens the program without taking the focus and returns its process id.
pub(crate) fn launch(launch: &Launch) -> Result<i32> {
    let program =
        fs::canonicalize(&launch.program).map_err(|e| Error::Launch(format!("{}: {e}", launch.program.display())))?;
    if program.extension().is_some_and(|e| e == "app") {
        launch_app(&program, launch)
    } else {
        launch_executable(&program, launch)
    }
}

fn launch_app(app: &Path, launch: &Launch) -> Result<i32> {
    let prefix = format!("{}/Contents/MacOS/", app.display());
    let before: HashSet<i32> = running_under(&prefix)?.into_iter().collect();
    open(app, launch)?;
    let started = Instant::now();
    loop {
        let new: Vec<i32> = running_under(&prefix)?.into_iter().filter(|pid| !before.contains(pid)).collect();
        match new.as_slice() {
            [pid] => return Ok(*pid),
            [] if started.elapsed() < LAUNCH_DEADLINE => thread::sleep(LAUNCH_POLL),
            [] => return Err(Error::Launch(format!("{} did not start", app.display()))),
            several => return Err(Error::Launch(format!("{} started more than once: {several:?}", app.display()))),
        }
    }
}

fn launch_executable(program: &Path, launch: &Launch) -> Result<i32> {
    let dir = wrapper_dir();
    let pid_file = dir.join("pid");
    let bundle = write_wrapper(&dir, program, &pid_file)?;
    open(&bundle, launch)?;
    wait_for_pid(&pid_file, program)
}

/// The wrapper stays in the system's temporary folder: deleting it while the program starts kills
/// the program, and nothing says when that start is over.
fn wrapper_dir() -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |t| t.as_nanos());
    std::env::temp_dir().join(format!("ghost-mouse-{}-{stamp}", process::id()))
}

fn write_wrapper(dir: &Path, program: &Path, pid_file: &Path) -> Result<PathBuf> {
    let bundle = dir.join("Target.app");
    let macos = bundle.join("Contents/MacOS");
    fs::create_dir_all(&macos)?;
    fs::write(bundle.join("Contents/Info.plist"), INFO_PLIST)?;
    let launcher = macos.join("launcher");
    fs::write(&launcher, launcher_script(&program.to_string_lossy(), &pid_file.to_string_lossy()))?;
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755))?;
    Ok(bundle)
}

fn wait_for_pid(pid_file: &Path, program: &Path) -> Result<i32> {
    let started = Instant::now();
    loop {
        // A missing or half-written file only means the launcher has not got there yet.
        if let Some(pid) = fs::read_to_string(pid_file).ok().and_then(|text| text.trim().parse().ok()) {
            return Ok(pid);
        }
        if started.elapsed() >= LAUNCH_DEADLINE {
            return Err(Error::Launch(format!("{} did not start", program.display())));
        }
        thread::sleep(LAUNCH_POLL);
    }
}

fn open(bundle: &Path, launch: &Launch) -> Result<()> {
    let mut command = Command::new("/usr/bin/open");
    command.args(["-g", "-n"]);
    for (name, value) in &launch.env {
        command.arg("--env").arg(format!("{name}={value}"));
    }
    command.arg(bundle);
    if !launch.args.is_empty() {
        command.arg("--args").args(&launch.args);
    }
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::Launch(String::from_utf8_lossy(&output.stderr).trim().to_string()))
}

fn running_under(prefix: &str) -> Result<Vec<i32>> {
    let output = Command::new("/bin/ps").args(["-axo", "pid=,comm="]).output()?;
    Ok(pids_under(&String::from_utf8_lossy(&output.stdout), prefix))
}

/// Quotes a value for a POSIX shell.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// The wrapper's launcher: records its process id, then becomes the real program, keeping the id.
pub(crate) fn launcher_script(program: &str, pid_file: &str) -> String {
    format!("#!/bin/sh\necho $$ > {}\nexec {} \"$@\"\n", shell_quote(pid_file), shell_quote(program))
}

/// Returns the ids of processes whose executable sits under `prefix`, from `ps -axo pid=,comm=`.
pub(crate) fn pids_under(ps_output: &str, prefix: &str) -> Vec<i32> {
    ps_output
        .lines()
        .filter_map(|line| {
            let (pid, command) = line.trim_start().split_once(' ')?;
            command.trim_start().starts_with(prefix).then(|| pid.parse().ok())?
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_survives_spaces_and_quotes() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("/Volumes/MACBOOK EXTERNO/a'b"), r"'/Volumes/MACBOOK EXTERNO/a'\''b'");
    }

    #[test]
    fn launcher_records_the_id_then_replaces_itself() {
        let script = launcher_script("/a b/app", "/t/pid");
        assert!(script.starts_with("#!/bin/sh\n"));
        let (record, run) =
            (script.find("echo $$ > '/t/pid'").unwrap(), script.find("exec '/a b/app' \"$@\"").unwrap());
        assert!(record < run);
    }

    #[test]
    fn pids_are_matched_by_executable_location() {
        let ps = "  101 /Applications/A.app/Contents/MacOS/A\n  202 /Applications/AB.app/Contents/MacOS/AB\n303 /usr/bin/x y\nbad line\n";
        assert_eq!(pids_under(ps, "/Applications/A.app/Contents/MacOS/"), vec![101]);
        assert!(pids_under(ps, "/nowhere/").is_empty());
    }
}
