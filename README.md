# ghost-mouse

Operates a desktop program from the outside without touching the real mouse. The cursor stays where the person left it, the app in front stays in front, and the person keeps working while the program is clicked, typed into and photographed.

It is a program of its own, not code inside the target: any program can be driven, whatever it was written in.

## Use

```bash
cargo build --release -p ghost-mouse
```

```bash
ghost-mouse open /System/Applications/Calculator.app
ghost-mouse windows --name Calculator
ghost-mouse click --pid 74110 --at 34,211
ghost-mouse drag --pid 74110 --from 272,312 --to 429,404
ghost-mouse scroll --pid 74110 --at 100,100 --dy 3
ghost-mouse key --pid 74110 cmd+shift+p
ghost-mouse type --pid 74110 "olá"
ghost-mouse shot --pid 74110 picture.png
```

Every command prints JSON when it has something to report, and on failure prints `ghost-mouse: <reason>` and exits with 1. Points are window points counted from the window's top-left corner, title bar included. `--window <id>` picks a window other than the program's main one; `shot` also reports the picture's scale, for converting a position read on the picture into a point to click.

As a library, `ghost_mouse::system()` gives the same operations.

## What it works on

Measured on macOS with another app in front: a Rust + Slint app, a native Mac app (Calculator) and an Electron app (VS Code). Clicks, drag, keys, shortcuts, accented text and pictures all land. Double, right and middle clicks, hover and scroll were measured on a test window behind the others.

## What it does not do

- Windows and Linux have no backend yet: every command answers that the system is not supported.
- The window of an opened program is a normal window on the desktop, so it can be seen. It is not hidden, only kept from coming forward.
- A program that brings itself forward as it starts (VS Code does) is in front for a moment, until `open` hands the front back. `open` only undoes the program itself, never a person moving to another app.
- It sees nothing inside the program but the picture, and an event has no receipt: confirm the effect with `shot`.
- Minimized windows and windows on another Space were not measured.

## Opening programs without their data

`open` starts a new copy of the program, with whatever arguments and variables are given, so a test copy can stay away from the real data:

```bash
ghost-mouse open "/Applications/Visual Studio Code.app" -- --user-data-dir /tmp/vs/d --extensions-dir /tmp/vs/e file.txt
```

```bash
ghost-mouse open --env HOME=/tmp/test-home ./my-program -- --flag
```

Keep an Electron data folder's path short: Electron keeps a socket inside it, and a path over 103 characters makes the program quit at once.

## How it works

Events are sent straight to one process (`CGEvent.postToPid` on macOS) instead of to the desktop, so the system neither moves the cursor nor changes the front app. Mouse events are built through `NSEvent` with the window number, or the program drops them for having no window. Pictures come from `screencapture -l<window>`, which sees a window under the others. A bare executable is opened through a throwaway `.app` wrapper, which is what lets macOS open it in the background.

## Tests

```bash
cargo test --workspace
```

```bash
GHOST_MOUSE_LIVE=1 cargo test -p ghost-mouse --test live
```

The second opens a real window behind the others and sends it every gesture.
