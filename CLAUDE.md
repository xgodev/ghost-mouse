# ghost-mouse

Operates desktop programs from the outside without touching the real mouse. See `README.md`.

## Rules

- Nothing here may move the real cursor or bring anything forward. The one exception is `Backend::activate`, used only to hand the front back after a program took it.
- `crates/core` knows no operating system: it holds the operations and checks. Each system lives behind `Backend`, in its own crate.
- Write the test first. Tests use a fake `Backend`; only `crates/ghost-mouse/tests/live.rs` opens a real window, and only with `GHOST_MOUSE_LIVE=1`.
- Before a commit: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
- Docs move with the code, in the same commit. `README.md` says only what was measured.
- Trying this on a real program: open a copy with its data kept away (`README.md`), never the person's own. Kill only the process id you opened.
