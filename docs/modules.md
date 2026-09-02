---
type: Reference
title: Module Reference
description: Per-module reference for the bella-engine and bella crates — purpose, key types, and public surface.
doc_id: modules
layer: [console]
project: bella
status: active
keywords: [module reference, bella-engine, bella crate, public API, Rust modules]
related: [capabilities, architecture, features]
---

# Module Reference

**What lives in each source file.** Use this when you know roughly what you need to change and want
to find the file, or when you are reviewing a module's public surface. It is a map, not a tutorial —
for the shape of the system read [`architecture.md`](architecture.md) first, and for what bella can
actually do read [`capabilities.md`](capabilities.md).

> **`bella-engine`'s public surface is a cross-repo contract.** `bastion` depends on this crate as
> an unpinned Cargo path dependency, with no cross-repo CI to catch a break. Before changing
> anything re-exported from `lib.rs`, or any public item in `browser.rs`, `theme.rs` or
> `markdown.rs`, build and test `bastion` as well. Rationale:
> `planning/decisions/D3-bella-engine-shared-with-bastion.md`.

## `bella-engine` crate

The render/layout library. Zero I/O — all functions take explicit parameters and return plain data. MIT derivative of `zemse/hackmd @ 7650cdc`; see `crates/bella-engine/ATTRIBUTION.md`.

### `lib.rs`

Crate root. Re-exports the stable public surface consumed by the `bella` app crate:

| Re-export | From |
|---|---|
| `body_pos`, `select_word_at` | `geometry` |
| `CheckboxMap`, `LinkMap`, `LinkTarget`, `TableMap` | `links` |
| `Rendered`, `render_with_edit` | `markdown` |
| `Frontmatter`, `FrontmatterValue`, `parse_frontmatter` | `frontmatter` |
| `Theme` | `theme` |

---

### `browser.rs`

Pure directory listing model.

**Key types:**

| Type | Description |
|---|---|
| `BrowserEntryKind` | `ParentDir`, `Dir`, `Markdown` |
| `BrowserEntry` | `path`, `display`, `kind` |
| `Browser` | `dir`, `entries`, `selected` (cursor), `scroll: u16`, `reveal_ignored: bool`, `dropped_entries: usize` |

**Key functions:**

| Function | What it does |
|---|---|
| `Browser::new(dir) -> Self` | List directory (ignore crate, gitignore-aware, max_depth 1, dotfiles skipped, symlinked children followed via `follow_links(true)`); `reveal_ignored` starts `false` |
| `Browser::set_reveal_ignored(reveal: bool)` | Flips `reveal_ignored` and re-lists `dir`, relaxing (or restoring) the `hidden`/`git_ignore`/`git_global`/`git_exclude` filters together; clamps `selected` into the new entry count |
| `Browser::move_cursor(delta, viewport_h)` | Wrap-around cursor with scroll-clamping |
| `Browser::descend() -> Option<PathBuf>` | Dir/ParentDir → path, else None |
| `Browser::ascend_target() -> Option<PathBuf>` | Parent of current dir |
| `resolve_corpus_root(invoked: &Path) -> PathBuf` | Walks up from the invoked path to the nearest `brain.toml`, else the nearest `.git` root, else falls back to the invoked path itself (a file's parent, since a file can never be a corpus root) |

Entry order: `..` (if parent exists) → subdirs (case-insensitive alpha) → `.md`/`.mdx` files (case-insensitive alpha). Non-markdown files are hidden. Invariant: `selected < entries.len()` (or 0 when empty); scroll kept so `scroll <= selected < scroll + viewport_h`. A walk error the `ignore` crate cannot resolve (e.g. a dangling symlink) is counted in `dropped_entries` rather than silently swallowed.

---

### `geometry.rs`

Pure screen-to-content coordinate conversion. No App dependency; every parameter is explicit.

**Key functions:**

| Function | What it does |
|---|---|
| `body_pos(viewport, line_numbers, line_count, scroll, col, row) -> Option<(usize, u16)>` | Convert screen `(col, row)` to `(content_row_index, local_col)`, accounting for gutter and scroll offset |
| `select_word_at(viewport, line_numbers, scroll, lines, col, row) -> Option<(String, usize, usize)>` | Return `(word, start_col, width)` for the word under a click |
| `point_in(rect, col, row) -> bool` | Test whether a screen point falls inside a Rect |

`select_word_at` walks left/right using unicode display widths and strips leading/trailing ASCII punctuation except `_-/.#` (preserves paths and identifiers intact).

---

### `markdown.rs`

The render pipeline. 2561 lines — the largest file in the project.

**Key types:**

| Type | Description |
|---|---|
| `Rendered` | Output struct: `lines: Vec<Line<'static>>`, `link_map`, `checkbox_map`, `table_map`, `images`, `blocks`, `headings`, `cursor_xy`, `row_source`, `frontmatter: Option<Frontmatter>` |
| `HeadingInfo` | Outline entry: `level` (1–6), `text`, `line` (display row index) |
| `BlockInfo` | Source-to-display mapping: `source_range`, `display_start/end` |
| `EditCtx` | Edit-mode cursor: `cursor` (source byte offset) |

**Key functions:**

| Function | What it does |
|---|---|
| `render(source, base_dir, width, theme) -> Rendered` | Convenience wrapper — no edit mode |
| `render_with_edit(source, base_dir, width, theme, edit, tables) -> Rendered` | Full render with optional edit-mode cursor support |

Pipeline: strip a leading OKF frontmatter fence (via [`frontmatter.rs`](#frontmatterrs), so pulldown-cmark never sees it — an unstripped fence misreads the closing `---` as a setext H2) → pulldown-cmark event stream → block tree → layout pass (word-wrap, link span extraction, table geometry). `render_with_edit` translates every byte offset it reports (`BlockInfo::source_range`, `row_source`, `EditCtx::cursor`) back to original-file coordinates, so callers never see stripped-body offsets. Edit mode replaces the smallest inline element containing the cursor with raw source text. Link spans that cross wrap boundaries are split — each physical line gets a separate `LinkSpan` entry.

Enabled extensions: `TABLES`, `STRIKETHROUGH`, `TASKLISTS`, `FOOTNOTES`, `SMART_PUNCTUATION`, `WIKILINKS`.

Raw HTML — including HTML comments (`<!-- ... -->`) — is dropped from the visible render (`Event::Html`/`Event::InlineHtml` produce no output), so sentinel comments in status/spec docs never surface as literal text. Regression coverage: `crates/bella-engine/tests/it/html_comments.rs`.

---

### `frontmatter.rs`

A restricted OKF frontmatter reader — deliberately not a general YAML parser and takes no YAML
crate dependency. It recognizes exactly four value shapes and never fails to parse a document; a
shape it doesn't specifically understand is retained verbatim rather than dropped or erroring.

**Key types:**

| Type | Description |
|---|---|
| `Frontmatter` | `entries: FrontmatterEntries`, `byte_range` (the whole `---`-fenced block, both fences plus trailing newline) |
| `FrontmatterValue` | `Scalar(String)` (bare or quoted), `List(Vec<String>)` (inline array or block list), `Raw(String)` (an unsupported shape, kept verbatim) |
| `FrontmatterEntries` | `Vec<(String, FrontmatterValue)>` — source order, not a `HashMap`, so the metadata pane renders keys in the order the author wrote them |

**Key functions:**

| Function | What it does |
|---|---|
| `detect_fence(source) -> Option<Range<usize>>` | Byte range of a leading `---`-fenced block, or `None` if the source doesn't open with one or never closes it |
| `parse(source) -> Option<Frontmatter>` | Detect + parse the four recognized value shapes; re-exported from `lib.rs` as `parse_frontmatter` |

The four recognized shapes: bare scalar (`type: Plan`), quoted scalar (`title: "a: b"`), inline
array (`related: [a, b]`), and a block list (`- item` lines indented under a key). Anything else —
detected by a value's leading YAML indicator character (`{ & * ! | >`) — becomes `Raw` rather than
an error.

---

### `links.rs`

Hyperlink resolution, anchor generation, and click-target hit-testing.

**Key types:**

| Type | Description |
|---|---|
| `LinkTarget` | `Url(String)`, `LocalFile(PathBuf)`, `Anchor(String)`, `FileAnchor(PathBuf, String)` |
| `LinkSpan` | `line`, `col_start`, `col_end`, `target` — one span per physical rendered line |
| `LinkMap` | `links: Vec<LinkSpan>`, `anchors: HashMap<String, usize>` (heading id → display line) |
| `CheckboxSpan` | Task-list item: `line`, `col_start`, `col_end`, `source_offset`, `checked` |
| `CheckboxMap` | `items: Vec<CheckboxSpan>` |
| `TableRegion` | Click geometry: column x-ranges, border lines, per-row y-ranges |
| `TableMap` | `regions: Vec<TableRegion>` |

**Key functions:**

| Function | What it does |
|---|---|
| `resolve(dest, base_dir) -> LinkTarget` | Resolve markdown link dest to typed target |
| `slugify(s) -> String` | GitHub-style anchor slug (lowercase, hyphens) |
| `LinkMap::at(line, col) -> Option<usize>` | Link index at position |
| `LinkMap::next_from/prev_from(line, col) -> Option<usize>` | Tab-order traversal |
| `CheckboxMap::at(line, col) -> Option<usize>` | Checkbox at position |
| `TableMap::hit(line, col) -> Option<(u64, TableHit)>` | Resolve click to table ID + granularity |

Table IDs are source byte offsets — stable across re-renders.

---

### `palette.rs`

Terminal color-depth detection and RGB → xterm-256 downgrade.

**Key types:**

| Type | Description |
|---|---|
| `ColorDepth` | `TrueColor`, `Indexed256` |

**Key functions:**

| Function | What it does |
|---|---|
| `depth() -> ColorDepth` | Detect once (OnceLock) via env-var probes |
| `rgb(r, g, b) -> Color` | Return `Color::Rgb` or nearest `Color::Indexed` depending on depth |

Detection priority: `COLORTERM` → `TERM_PROGRAM` (iTerm/vscode/WezTerm/ghostty/Hyper) → `TERM` suffix (`-direct`, `-truecolor`, `xterm-kitty`) → `Indexed256`. Apple Terminal always pinned to `Indexed256`. RGB → xterm-256 nearest-neighbour via squared Euclidean distance in the 6×6×6 cube + 24-step grayscale ramp.

---

### `syntax.rs`

Syntax highlighting via syntect.

**Key functions:**

| Function | What it does |
|---|---|
| `highlight(code, lang_token, theme_name, out)` | Push `Vec<Span<'static>>` per line into `out` |

`SyntaxSet` and `ThemeSet` are lazy-loaded via `OnceLock`. Falls back to raw text on highlight error. Uses `palette::rgb()` so output respects terminal color depth.

---

### `theme.rs`

> **Only `Theme::dark()` is reachable from the binary.** `resolve()`, `detect_terminal_theme()`,
> `light()` and `mission_control()` are tested but uncalled; `mission_control` has no name mapped to
> it in `resolve` at all.

Color themes — `Theme::dark()` carries bastiel's "cool-aurora" brand palette (ported from
`business/bastiel/src/app/globals.css` in the company-brain repo); `light()` is Catppuccin Latte,
unrelated and untouched by that port.

**Key types:**

| Type | Description |
|---|---|
| `Theme` | All semantic color roles: `fg`, `bg`, `muted`, `heading[6]`, `link`, `link_focused`, `code_fg`, `code_bg`, `status_fg`, `status_bg`, `syntect_theme`, … |

**Key functions:**

| Function | What it does |
|---|---|
| `Theme::dark() -> Self` | Bastiel cool-aurora — **the only one the binary constructs** |
| `Theme::light() -> Self` | Catppuccin Latte — reachable only through `resolve` |
| `Theme::mission_control() -> Self` | A third palette (indigo/violet/cyan, coordinated with bastion's console); **no name in `resolve` maps to it** |
| `resolve(name, cfg) -> Theme` | Pick by name or auto-detect via `COLORFGBG`. No call site. |

Auto-detect: `COLORFGBG` bg value 7–15 → light, 0–6 → dark.

---

### `md_config.rs`

Optional user config from `<config-dir>/md/config.toml`, where `<config-dir>` is whatever the
`dirs` crate resolves for the platform — `~/Library/Application Support` on macOS, `~/.config` on
Linux.

> **Dead code today.** `load()` has no call site anywhere in the workspace, so the file is never
> read and none of its three keys take effect. Same for `theme::resolve` below. See
> [`capabilities.md`](capabilities.md) § "Not wired up".

**Key types:**

| Type | Description |
|---|---|
| `Config` | `theme: Option<String>`, `width: Option<u16>`, `line_numbers: Option<bool>` |

**Key functions:**

| Function | What it does |
|---|---|
| `load() -> Config` | Read config file via `dirs` crate; return defaults on missing/parse failure |

---

## `bella` crate

The TUI binary. All terminal I/O and state management lives here.

### `main.rs`

CLI entry point and terminal lifecycle.

**Key types:**

| Type | Description |
|---|---|
| `Cli` | `file: Option<PathBuf>` — clap struct |

**Behaviour:**
1. Parse args via clap.
2. Install panic hook that restores terminal before re-raising.
3. Enable raw mode + alternate screen + mouse capture.
4. Call `run(terminal, file)`.
5. Restore terminal on exit (error or clean).

`run()` dispatches: no arg / directory arg → `App::new_browser`; file arg → `App::new`. Then calls `events::run_loop`.

---

### `app.rs`

Central state container. 2183 lines — the largest app-crate file.

**Key types:**

| Type | Description |
|---|---|
| `Mode` | `Reader`, `Browser` |
| `SearchState` | `query`, `matches: Vec<usize>` (display row indices), `current`, `input_mode` |
| `App` | All viewer state — see field table below |

**App fields (selected):**

| Field | Type | Purpose |
|---|---|---|
| `src` | `String` | Raw markdown source |
| `lines` | `Vec<Line<'static>>` | Rendered output |
| `link_map` | `LinkMap` | Hit-testable link spans |
| `scroll` | `usize` | Top visible display line |
| `viewport_height` | `u16` | Body height (updated each frame) |
| `width` | `u16` | Terminal width |
| `file` | `Option<PathBuf>` | Currently open file |
| `focused_link` | `Option<usize>` | Tab-focused link index |
| `hovered_link` | `Option<usize>` | Mouse-hovered link index |
| `toggled_checkboxes` | `HashSet<usize>` | Visual-only toggles (not persisted) |
| `search` | `SearchState` | Search state |
| `history` | `History` | Back/forward stack |
| `body_area` | `Rect` | Stored by draw_reader for mouse hit-testing |
| `drag_origin` | `Option<(usize, usize)>` | Drag-select start position |
| `selection` | `Option<Selection>` | Active text selection |
| `last_click` | `(Instant, (usize, u16))` | For double-click detection |
| `mode` | `Mode` | Current UI mode |
| `browser` | `Option<Browser>` | Browser state (Browser mode) |
| `browser_origin` | `Option<(PathBuf, usize)>` | Saved browser dir + cursor for round-trip |
| `render_worker` | `RenderWorker` | Background thread that runs `bella_engine::render_with_edit` off the event-loop thread |
| `render_generation` | `u64` | Token of the most recently requested render; used to discard stale results |
| `render_state` | `RenderState` | `Loading` (placeholder shown) or `Ready` (real content applied) for `render_generation` |

**Key invariants:**
- `render()`, `load_file()`, and `App::new()` kick off an async render via `render_worker.request_render(...)` and set `render_state = Loading` with placeholder `lines`; they never block on the render itself. `poll_render()` (called each tick of `run_loop`) drains the worker's channel and applies the result once it matches `render_generation`, discarding stale (superseded) ones.
- `render()` resets all overlay state (focused link, hovered link, search, selection, drag) because display line indices change.
- `last_click` is cleared after a successful double-click so triple-click starts fresh.
- `drag_origin` guards `selection_finish()` — set on Down, cleared by double-click, consumed by Up. Prevents double-calling finish on double-click sequences.
- Checkbox toggles are visual-only; never written to disk.

---

### `events.rs`

Event loop, key/mouse mappers, and action dispatcher.

**Key types:**

| Type | Description |
|---|---|
| `Action` | All possible state transitions — see table below |

**Action variants (selected):**

| Variant | Trigger |
|---|---|
| `ScrollDown/Up(u16)` | `j/k`, arrows, Ctrl-d/u, PageDown/Up, scroll wheel |
| `ToTop/Bottom` | `g`/`G` |
| `FocusNext/Prev` | `Tab`/`Shift-Tab` |
| `Follow` | `Enter` (reader) |
| `SearchStart/Char/Backspace/Commit/Next/Prev/Cancel` | `/` search mode |
| `HistoryBack/Forward` | `[`/`]` |
| `HoverAt{content_row, col}` | Mouse `Moved` |
| `DragStart/Update/End{content_row, col}` | Mouse `Down`/`Drag`/`Up` |
| `DoubleClickAt{screen_col, screen_row}` | Two `Down` events within 450 ms at same position |
| `BrowserUp/Down/Descend/Ascend` | Browser `j/k/Enter/Backspace` |
| `BrowserClickAt{row}` | Mouse `Down` in browser |
| `BrowserScroll(i32)` | Mouse scroll wheel in browser |
| `BrowserBack` | `Backspace` in reader mode |
| `Quit` | `q`, `Ctrl-C` |

**Key functions:**

| Function | What it does |
|---|---|
| `map_key(key, viewport_height) -> Action` | Reader key mapper (pure) |
| `map_browser_key(key) -> Action` | Browser key mapper (pure) |
| `map_search_key(key) -> Action` | Search input mapper (pure) |
| `map_mouse(mouse, app) -> Action` | Reader mouse mapper (pure) |
| `map_browser_mouse(mouse, app) -> Action` | Browser mouse mapper (pure) |
| `apply(action, app)` | Dispatch action → App state mutations |
| `run_loop(terminal, app) -> Result<()>` | Main event loop |

`run_loop` polls for terminal events with a timeout (`EVENT_POLL_TIMEOUT`, 50ms) rather than blocking on `event::read()`, and calls `app.poll_render()` each tick to drain any background render result that has landed. This keeps input responsive while `render_worker.rs` (see below) parses a document off-thread.

---

### `render_worker.rs`

Background render thread: offloads `bella_engine::render_with_edit` off the event-loop thread so a large document never stalls input handling. Owns no TUI state and does not touch `bella-engine`.

**Key types:**

| Type | Description |
|---|---|
| `RenderWorker` | Owns the request/response `mpsc` channels to the background thread |
| `RenderResult` | `generation: u64`, `rendered: Rendered` — a delivered render tagged with its request's generation |

**Key functions:**

| Function | What it does |
|---|---|
| `RenderWorker::spawn() -> Self` | Spawn the background thread; returns immediately |
| `request_render(source, base_dir, width, theme, edit, tables) -> u64` | Send a render request; returns a monotonically increasing generation token, never blocks |
| `try_recv_latest() -> Option<RenderResult>` | Non-blocking drain; returns only the highest-generation result currently buffered, discarding older ones |
| `recv_blocking() -> Result<RenderResult, RecvError>` | Blocking receive of the next result (test-only synchronous waiting) |
| `is_latest(result, generation) -> bool` | Whether `result` matches the given generation (i.e. is not stale) |

The worker thread processes requests strictly in FIFO order over a single `std::thread`; the caller (`App`) is responsible for comparing each delivered `RenderResult`'s generation against its own `render_generation` and discarding stale ones (see `app.rs` above).

---

### `history.rs`

Back/forward navigation stack.

**Key types:**

| Type | Description |
|---|---|
| `HistoryEntry` | `path: PathBuf`, `scroll: u16` |
| `History` | `entries: Vec<HistoryEntry>`, `cursor: Option<usize>` |

**Key functions:**

| Function | What it does |
|---|---|
| `push(entry)` | Append + advance cursor; truncate forward tail |
| `back() -> Option<&HistoryEntry>` | Move cursor back; return entry |
| `forward() -> Option<&HistoryEntry>` | Move cursor forward; return entry |
| `can_back/can_forward() -> bool` | Precondition checks |

Standard browser model: push at mid-stack truncates forward history.

---

### `selection.rs`

Text selection and clipboard integration.

**Key types:**

| Type | Description |
|---|---|
| `Selection` | `anchor: (usize, usize)`, `cursor: (usize, usize)` — both `(row, char_col)` |

**Key functions:**

| Function | What it does |
|---|---|
| `Selection::normalized() -> ((usize,usize),(usize,usize))` | Document-order start/end |
| `extract_text(lines, sel) -> String` | Multi-row text extraction, joined with `\n` |
| `copy_to_clipboard(text) -> Result<(), String>` | Copy via arboard; non-fatal |

Columns are char counts (not bytes); out-of-bounds clamped.

---

### `ui.rs`

Ratatui draw functions. No state mutation — read-only access to App.

**Key functions:**

| Function | What it does |
|---|---|
| `draw_reader(frame, area, app) -> u16` | Draw body + status line; return body height |
| `draw_browser(frame, area, app)` | Draw bordered directory listing |

**Overlay stack in `draw_reader` (later = higher z-order):**
1. Search matches — Yellow bg
2. Hovered link — Cyan underline
3. Focused link — Reversed (wins on overlap with hover)
4. Toggled checkboxes — glyph swap in-place
5. Selection — LightBlue bg (topmost)

**Browser styling:** `ParentDir`/`Dir` = bold cyan; `Markdown` = default fg; selected row prefixed with `▶ `.

**Status line content (priority order):**
1. Search input mode: `/query [M/N]`
2. Status message (e.g. clipboard error): shown for one frame
3. Default: `bella · filename · scroll/total`
