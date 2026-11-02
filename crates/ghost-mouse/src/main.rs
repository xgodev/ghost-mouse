//! Command line of ghost-mouse: parses one command and prints its result as JSON.

use std::{path::PathBuf, process::ExitCode, time::Duration};

use clap::{Args, Parser, Subcommand};
use ghost_mouse::{system, Button, Chord, Launch, Point, Query, Result, Window};
use serde_json::json;

/// Clicks, drags, types and takes pictures of a program, without touching the real mouse.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Target {
    /// Process to drive.
    #[arg(long)]
    pid: i32,
    /// Window id from `windows`; defaults to the program's main window.
    #[arg(long)]
    window: Option<u32>,
}

/// Where the pointer goes: a point, or the centre of a control found by name.
#[derive(Args)]
struct Spot {
    /// Point in the window, in points from its top-left corner.
    #[arg(long, value_name = "X,Y", required_unless_present = "name", conflicts_with = "name")]
    at: Option<Point>,
    /// The control's name, exactly as `controls` prints it.
    #[arg(long)]
    name: Option<String>,
    /// The control's role, exactly as `controls` prints it, such as AXButton.
    #[arg(long, requires = "name", conflicts_with = "at")]
    role: Option<String>,
    /// Which of several matching controls, counting from 1 in the order `controls` lists them.
    #[arg(long, requires = "name", conflicts_with = "at")]
    nth: Option<usize>,
    /// How long to wait for the control to appear; none by default.
    #[arg(long, requires = "name", conflicts_with = "at")]
    wait_secs: Option<u64>,
}

impl Spot {
    fn query(&self) -> Query {
        Query { name: self.name.clone(), role: self.role.clone() }
    }

    fn wait(&self) -> Duration {
        Duration::from_secs(self.wait_secs.unwrap_or(0))
    }
}

#[derive(Subcommand)]
enum Command {
    /// Opens a program in the background and prints its process id and main window.
    Open {
        /// A .app bundle or an executable.
        program: PathBuf,
        /// Environment variable for the program, as NAME=VALUE.
        #[arg(long = "env", value_parser = parse_env)]
        env: Vec<(String, String)>,
        /// How long to wait for the window.
        #[arg(long, default_value_t = 30)]
        wait_secs: u64,
        /// Arguments for the program.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Lists windows.
    Windows {
        #[arg(long)]
        pid: Option<i32>,
        /// Only programs whose name contains this text.
        #[arg(long)]
        name: Option<String>,
    },
    /// Lists the window's interface elements as the program describes them to assistive tools.
    Controls {
        #[command(flatten)]
        target: Target,
        /// Only elements with exactly this name.
        #[arg(long)]
        name: Option<String>,
        /// Only elements with exactly this role, such as AXButton.
        #[arg(long)]
        role: Option<String>,
        /// How long to wait for at least one element to match.
        #[arg(long, default_value_t = 0)]
        wait_secs: u64,
    },
    /// Rests the pointer over a point or a control of the window.
    Hover {
        #[command(flatten)]
        target: Target,
        #[command(flatten)]
        spot: Spot,
    },
    /// Clicks a point of the window, in points from its top-left corner, or a control found by name.
    Click {
        #[command(flatten)]
        target: Target,
        #[command(flatten)]
        spot: Spot,
        /// left, right or middle.
        #[arg(long, default_value = "left")]
        button: Button,
        #[arg(long, default_value_t = 1)]
        count: u8,
    },
    /// Drags between two points of the window.
    Drag {
        #[command(flatten)]
        target: Target,
        #[arg(long, value_name = "X,Y")]
        from: Point,
        #[arg(long, value_name = "X,Y")]
        to: Point,
        #[arg(long, default_value = "left")]
        button: Button,
        /// Points passed through on the way.
        #[arg(long, default_value_t = 12)]
        steps: u32,
    },
    /// Scrolls at a point; positive goes toward the end of the content.
    Scroll {
        #[command(flatten)]
        target: Target,
        #[arg(long, value_name = "X,Y")]
        at: Point,
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        dx: i32,
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        dy: i32,
    },
    /// Presses a key combination such as return or cmd+shift+a.
    Key {
        #[command(flatten)]
        target: Target,
        chord: Chord,
    },
    /// Types text.
    Type {
        #[command(flatten)]
        target: Target,
        text: String,
    },
    /// Takes a picture of the window, even when covered, and prints how pixels map to points.
    Shot {
        #[command(flatten)]
        target: Target,
        out: PathBuf,
    },
    /// Completes the program's native file panel with a path and prints the path chosen.
    ChooseFile {
        #[arg(long)]
        pid: i32,
        /// In an open panel the file to choose; in a save panel the new file, which must not exist.
        path: PathBuf,
        /// How long to wait for the panel, and for it to reach the path.
        #[arg(long, default_value_t = 60)]
        wait_secs: u64,
    },
    /// Cancels the program's native file panel.
    CancelFile {
        #[arg(long)]
        pid: i32,
        /// How long to wait for the panel.
        #[arg(long, default_value_t = 30)]
        wait_secs: u64,
    },
}

fn parse_env(text: &str) -> std::result::Result<(String, String), String> {
    let (name, value) = text.split_once('=').ok_or_else(|| format!("{text} is not NAME=VALUE"))?;
    Ok((name.to_string(), value.to_string()))
}

fn window(target: &Target) -> Result<Window> {
    match target.window {
        Some(id) => system().window(target.pid, id),
        None => system().main_window(target.pid),
    }
}

fn run(command: Command) -> Result<Option<serde_json::Value>> {
    let ghost = system();
    match command {
        Command::Open { program, env, wait_secs, args } => {
            let window = ghost.open(&Launch { program, args, env }, Duration::from_secs(wait_secs))?;
            Ok(Some(json!({ "pid": window.pid, "window": window })))
        }
        Command::Windows { pid, name } => {
            let mut windows = ghost.windows(pid)?;
            if let Some(name) = name {
                windows.retain(|w| w.owner.contains(&name));
            }
            Ok(Some(json!(windows)))
        }
        Command::Controls { target, name, role, wait_secs } => {
            let query = Query { name, role };
            Ok(Some(json!(ghost.controls(&window(&target)?, &query, Duration::from_secs(wait_secs))?)))
        }
        Command::Hover { target, spot } => {
            let window = window(&target)?;
            match spot.at {
                Some(at) => ghost.hover(&window, at),
                None => ghost.hover_control(&window, &spot.query(), spot.nth, spot.wait()).map(drop),
            }
            .map(|()| None)
        }
        Command::Click { target, spot, button, count } => {
            let window = window(&target)?;
            match spot.at {
                Some(at) => ghost.click(&window, at, button, count),
                None => ghost.click_control(&window, &spot.query(), spot.nth, button, count, spot.wait()).map(drop),
            }
            .map(|()| None)
        }
        Command::Drag { target, from, to, button, steps } => {
            ghost.drag(&window(&target)?, from, to, button, steps).map(|()| None)
        }
        Command::Scroll { target, at, dx, dy } => ghost.scroll(&window(&target)?, at, dx, dy).map(|()| None),
        Command::Key { target, chord } => ghost.press(&window(&target)?, chord).map(|()| None),
        Command::Type { target, text } => ghost.type_text(&window(&target)?, &text).map(|()| None),
        Command::Shot { target, out } => Ok(Some(json!(ghost.shot(&window(&target)?, &out)?))),
        Command::ChooseFile { pid, path, wait_secs } => {
            let chosen = ghost.choose_file(pid, &path, Duration::from_secs(wait_secs))?;
            Ok(Some(json!({ "path": chosen })))
        }
        Command::CancelFile { pid, wait_secs } => {
            ghost.cancel_file_panel(pid, Duration::from_secs(wait_secs)).map(|()| None)
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(Some(value)) => {
            println!("{value:#}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ghost-mouse: {error}");
            ExitCode::FAILURE
        }
    }
}
