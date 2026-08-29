# Bella

A terminal markdown viewer with full mouse support — scroll, click links, drag-select text and
copy to your system clipboard, all from inside a `ratatui` TUI. Local-only: it reads files off
disk and opens a system browser tab for external links; nothing else talks to the network.

> Part of the **Bastion** ecosystem — see the [bastion-os](https://github.com/bredmond1019/bastion-os)
> front door for the wider architecture. Bella itself works standalone; nothing below requires it.

## What this is for

You want to read (or browse a folder of) markdown files without leaving the terminal — with
proper link-following, in-document search, and mouse selection, instead of `less`/`cat`-ing raw
markdown source. Point it at a file, or at nothing, and it opens a file browser.

## Quickstart

Run these in a shell:

```bash
# 1. Clone and build
git clone https://github.com/bredmond1019/bella && cd bella
cargo build --release

# 2. Open a specific file
cargo run --release -p bella -- README.md

# 3. Or browse a directory (defaults to the current directory if you omit the argument)
cargo run --release -p bella
cargo run --release -p bella -- some/dir
```

Press `q` or `Ctrl-C` to quit from either mode.

To install the `bella` binary onto your `PATH` instead of running it via `cargo run`:

```bash
cargo install --path crates/bella
```

### Prerequisites

| Requirement | Why | If missing |
|---|---|---|
| Rust stable toolchain, edition 2024 | Building the workspace | Install via [rustup](https://rustup.rs/) |
| A terminal emulator with mouse support | Hover/click/drag gestures | Most modern emulators work (iTerm2, WezTerm, kitty, Alacritty, Ghostty); without one, keyboard navigation still works fully |
| A system clipboard provider | Drag-select / double-click copy | On Linux without a display server, clipboard writes fail; the error shows in the status line for one frame and clears — everything else still works |

## Keybindings

### Reader mode (a file is open)

| Key | Action |
|---|---|
| `j` / `↓` | Scroll down 1 line |
| `k` / `↑` | Scroll up 1 line |
| `Ctrl-d` | Scroll down half a page |
| `Ctrl-u` | Scroll up half a page |
| `PageDown` | Scroll down a full page |
| `PageUp` | Scroll up a full page |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `Tab` | Focus next link |
| `Shift-Tab` | Focus previous link |
| `Enter` | Follow the focused link (local file, in-file anchor, or system browser) |
| `[` | History back |
| `]` | History forward |
| `/` | Start in-document search |
| `n` / `N` | Next / previous search match |
| `Esc` | Clear link focus, or cancel an in-progress search |
| `Backspace` | Return to the file browser |
| `q` / `Ctrl-C` | Quit |

### Browser mode (no file open — navigating a directory)

| Key | Action |
|---|---|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Enter` | Open the selected file, or descend into the selected directory |
| `Backspace` | Ascend to the parent directory |
| `q` / `Ctrl-C` | Quit |

### Mouse — reader mode

| Gesture | Action |
|---|---|
| Scroll wheel | Scroll up / down, 3 lines per tick |
| Hover | Highlight the link under the pointer |
| Click a link | Follow it |
| Click a checkbox | Toggle its visual state (display-only — does not edit the source file) |
| Click + drag | Select text; releasing copies the selection to the system clipboard |
| Double-click | Select the word under the pointer (within a 450 ms window) and copy it |

### Mouse — browser mode

| Gesture | Action |
|---|---|
| Scroll wheel | Scroll the entry list |
| Click an entry | Select it, then immediately open the file or descend into the directory |

Every capability with how to invoke it, including what markdown bella does and does not render:
[`docs/capabilities.md`](docs/capabilities.md). The same gestures with the internal call chain
behind each: [`docs/features.md`](docs/features.md).

## Directory map

```
bella/
└── crates/
    ├── bella-engine/   ← render/layout library: markdown parsing, syntax highlighting,
    │                      link resolution, theme/palette, checkbox + table geometry
    └── bella/          ← TUI binary: clap CLI, ratatui draw loop, event dispatch,
                           app state, file browser, text selection
```

`bella-engine` has zero terminal or I/O dependencies — every function takes explicit parameters
and returns plain data, so it can be (and is) unit-tested without a live terminal. See
[`docs/architecture.md`](docs/architecture.md) for the full render pipeline and the reasoning
behind the two-crate split.

## Theming

Bella renders with the **Catppuccin Mocha** (dark) palette. Colour *depth* is detected from your
environment — `COLORTERM`, then `TERM_PROGRAM`, then `TERM` — and RGB values are downgraded to the
nearest xterm-256 colour on terminals without truecolor support.

> **Light mode is not selectable yet.** A Latte (light) palette and a `config.toml` loader both
> exist in `bella-engine`, but nothing in the binary calls them — the app constructs the dark theme
> directly. Treat light mode and the config file as unshipped; see
> [`docs/capabilities.md`](docs/capabilities.md) § "Not wired up" for the exact state of each.

## Development

```bash
cargo test                              # full test suite
cargo clippy --all-targets -- -D warnings   # lint gate
cargo fmt --check                       # format check
```

Prerequisites, test-layer breakdown, and the full contributor workflow:
[`docs/development.md`](docs/development.md).

## Troubleshooting

| Symptom | Likely cause | What to check |
|---|---|---|
| Mouse clicks/drags do nothing | Terminal emulator doesn't forward mouse events, or you're over SSH without mouse passthrough | Try a different emulator; keyboard-only navigation always works |
| Copy-on-select silently does nothing | No clipboard provider available (e.g. headless Linux) | Look for the one-frame error in the status line; this is expected in that environment |
| Colours look wrong / washed out | Terminal not reporting truecolor support | Set `COLORTERM=truecolor` if your emulator supports it |
| Everything renders dark on a light terminal | Expected — bella is dark-only today | There is no light-mode switch yet; see [`docs/capabilities.md`](docs/capabilities.md) § "Not wired up" |
| Raw HTML in a document renders as nothing | Expected — HTML events are dropped by the renderer | Use markdown equivalents; see [`docs/capabilities.md`](docs/capabilities.md) § "Markdown bella renders" |
| An image shows as `[image: path]` | Expected — images are placeholders, not rendered | No terminal image protocol is implemented |

## Roadmap / known limitations

- **Editing:** Bella is a viewer today — there is no edit mode. An editor mode with full mouse
  support is planned.
- **Config & theming:** the `config.toml` loader and the light palette are written but not wired
  into the binary. Wiring them up — plus live reload and richer theme options — is planned.
- **Console absorption:** Bella is planned to eventually fold into a unified operator-console
  binary rather than remain a standalone app; the standalone binary will keep working either way.

## See also

- [`docs/capabilities.md`](docs/capabilities.md) — every capability and how to invoke it, derived from source
- [`docs/architecture.md`](docs/architecture.md) — two-crate design, render pipeline, event loop, coordinate system
- [`docs/modules.md`](docs/modules.md) — per-module reference: purpose, key types, public functions
- [`docs/development.md`](docs/development.md) — prerequisites, build/test/lint, contributor workflow
- [`docs/features.md`](docs/features.md) — every keybinding and mouse gesture with the internal call chain
- [`crates/bella-engine/ATTRIBUTION.md`](crates/bella-engine/ATTRIBUTION.md) — which files were ported from `zemse/hackmd`, and what changed

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) · <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) · <http://opensource.org/licenses/MIT>)

at your option — **with one exception**. The `bella-engine` crate is a derivative of
[zemse/hackmd](https://github.com/zemse/hackmd) (commit `7650cdc`) and stays **MIT only**, with the
upstream copyright notice preserved; see
[`crates/bella-engine/ATTRIBUTION.md`](./crates/bella-engine/ATTRIBUTION.md) for the ported files.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.

Built for one operator and released because it may be useful to others — there is no support
obligation, no issue-response SLA, and no stability promise.
