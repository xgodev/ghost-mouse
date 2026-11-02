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
ghost-mouse controls --pid 74110 --role AXButton
ghost-mouse click --pid 74110 --name 7
ghost-mouse drag --pid 74110 --from 272,312 --to 429,404
ghost-mouse scroll --pid 74110 --at 100,100 --dy 3
ghost-mouse key --pid 74110 cmd+shift+p
ghost-mouse type --pid 74110 "olá"
ghost-mouse shot --pid 74110 picture.png
ghost-mouse choose-file --pid 74110 ~/report.pdf
ghost-mouse cancel-file --pid 74110
```

Every command prints JSON when it has something to report, and on failure prints `ghost-mouse: <reason>` and exits with 1. Points are window points counted from the window's top-left corner, title bar included. `--window <id>` picks a window other than the program's main one; `shot` also reports the picture's scale, for converting a position read on the picture into a point to click.

As a library, `ghost_mouse::system()` gives the same operations.

## Controls by name

`controls` lists the window's interface as the program describes it to assistive tools: for each element its role as the system names it (`AXButton`, `AXCheckBox`, `AXStaticText`...), its name (the title, or the description when there is no title), its value, the states the program reports (`enabled`, `focused`, `selected`, `checked`, `expanded`), its `frame` in window points, and whether it is `visible`. `--name` and `--role` keep only the elements with exactly that name and role; `--wait-secs` waits for at least one to match.

`click --name` and `hover --name` aim at the centre of the control's frame and go the same way as `--at`, pointer rest included. `--role` narrows the search and `--nth` picks among several matches, counting from 1 in the order `controls` lists them. No match fails with the controls the window has, several matches without `--nth` fail with each one and its position, and a control outside the visible part of the window or of a scrolling area is refused until it is scrolled into view. Reading the controls needs the Accessibility permission for whatever runs the command, and says so when it is missing.

Measured on macOS 26 with each program behind another app, the cursor and the front app untouched:

- Calculator: 37 elements; `7`, `Adicionar`, `8` and `Igual a` clicked by name gave 15. Names come in the system's language.
- A test window: a button, a checkbox (`checked` follows it), two buttons with the same name picked with `--nth`, and a button scrolled out of sight, refused.
- A web page in a window: its button found by name and pressed.
- VS Code 1.143.1: 368 to 399 elements as its panels change. Electron shows its page to assistive tools only when asked; `controls` asks once and waits for it (about 2 s the first time), and it stays on until the program quits. Once asked, VS Code shows its own "screen reader" notice. Its side bar items Explorer, Search, Source Control and Extensions were clicked by name, 7 of 8 landing; the one that missed was the first, right after the page was asked for. Each also has a group of the same name over it, so `--role AXRadioButton` picks the item.
- JoVee Lut 0.1.0-beta.3, a Rust + Slint app: 49 elements. Read right after the program opened it held 5 unnamed ones, so a click by name there needs `--wait-secs`. Its seven `Começar` buttons are refused without `--nth`; `--nth 1` opened the first step and `Voltar` came back.

## File panels

`choose-file` completes the native open or save panel a program is showing, window or sheet, and prints the path chosen. `cancel-file` cancels it. Both wait for the panel to appear (`--wait-secs`) and answer only once it has closed. They need the Accessibility permission for whatever runs them, and say so when it is missing.

Keys sent to a process do not reach this panel, because it lives in a system service, so it is operated through Accessibility instead: rows are found by the file they stand for and controls by identifier, in any language.

Measured on macOS 26 with a test program behind another app: the cursor did not move, the program never became active in 6 runs of the command, and files were chosen, saved and cancelled. In earlier runs a program took the front in 1 of 6, so it may be in front for a moment until the front is handed back; only the program itself is undone, never a person moving to another app.

- A nearby file takes about 4 s, 3 of them watching the front after the panel closes. The panel is walked through its own list, so a far path takes longer: 27 s to reach a file in the home folder from a folder under `/private/tmp`, passing through a list of 700 rows.
- A path the panel does not list confirms nothing and leaves the panel open: `the file panel does not list <path>`. That includes hidden folders, which can only be entered from inside (a file under `/private/tmp` was not reached from `~/Documents`), and it takes the whole `--wait-secs` when the walk runs out of time.
- A file the panel filters out by type is refused, and the panel stays open.
- A save panel never replaces a file: a path that already exists is refused before anything is touched. The folder must exist.
- Only the list view was measured; other views and a collapsed save panel were not. Going through folders adds them to the panel's recents, as using it by hand does.

## What it works on

Measured on macOS with another app in front: a Rust + Slint app, a native Mac app (Calculator) and an Electron app (VS Code). Clicks, drag, keys, shortcuts, accented text and pictures all land. Double, right and middle clicks, hover and scroll were measured on a test window behind the others.

A click rests the pointer on the point for 300 ms before pressing, because a program drawing its own buttons only takes the press once the pointer has rested: on Creality Print's Slice plate, 100 ms missed and 200 ms hit.

A single left click on a web page inside a window goes through Accessibility, which needs that permission and may take up to 2 s the first time. The page gets the click but no hover. Measured on a test page and on a button of Creality Print's Send to Lan Printer page; its Start Print was not pressed, because a real printer was selected. A switch on that same page (Print Calibration) did not change, so such a press is not proven for every control.

### Lists a program opens

A list a program opens as its own panel can stay off the screen while the program is not in front. It is open all the same, so it can be photographed and clicked:

```bash
ghost-mouse click --pid 74110 --at 833,295
ghost-mouse windows --pid 74110
ghost-mouse shot --pid 74110 --window 951 list.png
ghost-mouse click --pid 74110 --window 951 --at 100,90
```

`windows --pid` shows it with a `layer` other than 0 and, while it is off the screen, `held_back: true`; a list on the screen shows `on_screen: true` instead. It is never taken for the main window. Telling an open list from the panels a program never opened takes a private tag of the window server (`SLSGetWindowTags`, measured on macOS 26); without it, `held_back` stays false. Measured on Creality Print's profile list, behind another app: the list was read from its picture, a row was chosen and the program's layer height followed it.

## What it does not do

- Windows and Linux have no backend yet: every command answers that the system is not supported.
- The window of an opened program is a normal window on the desktop, so it can be seen. It is not hidden, only kept from coming forward.
- A program that brings itself forward as it starts (VS Code does) is in front for a moment, until `open` hands the front back. `open` only undoes the program itself, never a person moving to another app.
- It sees nothing inside the program but the picture, and an event has no receipt: confirm the effect with `shot`.
- Minimized windows and windows on another Space were not measured.
- macOS numbers each program it starts, and each start adds to the count. Past 32 bits (measured after about a million starts in 21 days, with a tool calling `osascript` every second), what it opens stays stopped and events sent to any new program are dropped, while pictures and accessibility presses still work. `open` then answers that the program is held stopped and every gesture answers that events are not delivered, until the Mac restarts.

## Opening programs without their data

`open` starts a new copy of the program, with whatever arguments and variables are given, so a test copy can stay away from the real data:

```bash
ghost-mouse open "/Applications/Visual Studio Code.app" -- --user-data-dir /tmp/vs/d --extensions-dir /tmp/vs/e file.txt
```

```bash
ghost-mouse open --env HOME=/tmp/test-home ./my-program -- --flag
```

Variables reach a bare executable even when macOS would drop them, such as `DYLD_FALLBACK_LIBRARY_PATH`. Keep an Electron data folder's path short: Electron keeps a socket inside it, and a path over 103 characters makes the program quit at once.

## How it works

Events are sent straight to one process (`CGEvent.postToPid` on macOS) instead of to the desktop, so the system neither moves the cursor nor changes the front app. Mouse events are built through `NSEvent` with the window number, or the program drops them for having no window. Pictures come from `screencapture -l<window>`, which sees a window under the others. A bare executable is opened through a throwaway `.app` wrapper, which is what lets macOS open it in the background.

## Tests

```bash
cargo test --workspace
```

```bash
GHOST_MOUSE_LIVE=1 cargo test -p ghost-mouse --test live
```

The second opens a real window behind the others and sends it every gesture, and completes real file panels. Run with `-- --test-threads=1`, as the panel tests watch the front app. Test programs are built in the system's temporary folder: macOS asks before a new program reads an external disk, so on a build folder there every test program waits for that answer and never shows its window.

## License

MIT, see `LICENSE`.
