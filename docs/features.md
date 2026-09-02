---
type: Reference
title: Feature Reference
description: All keybindings and mouse gestures with descriptions of what happens internally.
doc_id: features
layer: [console]
project: bella
status: active
keywords: [keybindings, mouse gestures, reader mode, browser mode, keyboard shortcuts]
related: [capabilities, architecture, modules]
---

# Feature Reference

**This page is the wiring diagram, not the user guide.** Every keybinding and mouse gesture is
listed with the `Action` it produces and the `App` method that handles it — the reference you want
when you are changing behaviour, not when you are trying to use bella.

If you just want to know what bella can do and how to invoke it, read
[`capabilities.md`](capabilities.md) instead. If you are adding a keybinding, the four places you
have to touch are listed in [`development.md`](development.md#adding-a-new-keybinding).

## Reader Mode

Reader mode is active when a `.md` or `.mdx` file is open. The status line shows `bella · filename · scroll/total` when idle.

### Keyboard — Reader Mode

| Key | Action | What happens internally |
|---|---|---|
| `j` / `↓` | Scroll down 1 line | `ScrollDown(1)` → `App::scroll_down(1)` → `scroll = min(scroll+1, max_scroll())` |
| `k` / `↑` | Scroll up 1 line | `ScrollUp(1)` → `App::scroll_up(1)` |
| `Ctrl-d` | Scroll down half-page | `ScrollDown(viewport_height/2)` |
| `Ctrl-u` | Scroll up half-page | `ScrollUp(viewport_height/2)` |
| `PageDown` | Scroll down full page | `ScrollDown(viewport_height)` |
| `PageUp` | Scroll up full page | `ScrollUp(viewport_height)` |
| `g` | Go to top | `ToTop` → `scroll = 0` |
| `G` | Go to bottom | `ToBottom` → `scroll = max_scroll()` |
| `Tab` | Focus next link | `FocusNext` → `App::focus_next()` → advances `focused_link` index; scrolls to keep link visible |
| `Shift-Tab` | Focus previous link | `FocusPrev` → `App::focus_prev()` |
| `Enter` | Follow focused link | `Follow` → `App::follow_focused()` → resolves `LinkTarget`, dispatches on type (see Link Following below) |
| `[` | History back | `HistoryBack` → `App::go_back()` → loads previous file at saved scroll position |
| `]` | History forward | `HistoryForward` → `App::go_forward()` |
| `/` | Start search | `SearchStart` → `app.search.input_mode = true`; status line shows `/` prompt |
| `n` | Next search match | `SearchNext` → advance `search.current`; scroll to match line |
| `N` | Previous search match | `SearchPrev` |
| `Esc` | Clear focus / cancel search | `ClearFocus` (normal) or `SearchCancel` (search mode) → clear `focused_link`, `hovered_link`, end search input |
| `Backspace` | Return to file browser | `BrowserBack` → `App::back_to_browser()` → restore saved browser dir + cursor; switch to Browser mode |
| `q` / `Ctrl-C` | Quit | `Quit` → `app.should_quit = true` |

### Keyboard — Search Mode

Triggered by `/`. Status line shows `/query [M/N]` where M is current match index and N is total.

| Key | Action |
|---|---|
| Any printable char | Append to query; run incremental search |
| `Backspace` | Remove last char from query; re-run search |
| `Enter` | Commit search; exit input mode (keep matches visible) |
| `Esc` | Cancel search; clear query and highlights |

After committing, `n`/`N` cycle matches and `Esc` clears.

### Mouse — Reader Mode

| Gesture | Action | What happens internally |
|---|---|---|
| Scroll wheel up | Scroll up 3 lines | `ScrollUp(3)` |
| Scroll wheel down | Scroll down 3 lines | `ScrollDown(3)` |
| Hover over link | Highlight link | `HoverAt{content_row, col}` → `App::hover_at()` → sets `hovered_link`; `ui.rs` applies Cyan underline style |
| Hover off link | Clear highlight | `HoverAt` with no hit → `hovered_link = None` |
| Click link | Follow link | `DragStart` + `DragEnd` (no drag) → `App::click_at()` → same dispatch as `Follow` |
| Click checkbox | Toggle visual state | `App::click_at()` → `CheckboxMap::at()` hit → add/remove from `toggled_checkboxes`; glyph swap at draw time |
| Click + drag | Select text | `DragStart` → `App::selection_start()`; `DragUpdate` → `App::selection_update()`; `DragEnd` → `App::selection_finish()` → `extract_text` → `copy_to_clipboard` via arboard |
| Double-click | Select word | Two `Down` within 450 ms at same content position → `DoubleClickAt` → `App::double_click_word_select()` → `select_word_at` → `copy_to_clipboard` |

#### Link Following

`LinkTarget` dispatch in `App::follow_focused()` / `App::click_at()`:

| Target type | What happens |
|---|---|
| `Url(url)` | `open::that(url)` — opens in system browser |
| `LocalFile(path)` | `App::load_file(path)` — load + render in-place; push previous position to history |
| `Anchor(id)` | `LinkMap::anchors.get(id)` → `scroll_to_line(target_line)` |
| `FileAnchor(path, id)` | Load file, then scroll to anchor |

History is pushed on file navigation; anchor-only navigation does not add a history entry.

---

## Browser Mode

Browser mode is active when no file is open (startup with no arg or a directory arg). A bordered pane shows the directory listing. `▶` marks the selected entry. Directories and `..` are shown in bold cyan; markdown files in default colour.

### Keyboard — Browser Mode

| Key | Action | What happens internally |
|---|---|---|
| `j` / `↓` | Move cursor down | `BrowserDown` → `Browser::move_cursor(+1, viewport_h)` → wrap-around + scroll clamp |
| `k` / `↑` | Move cursor up | `BrowserUp` → `Browser::move_cursor(-1, viewport_h)` |
| `Enter` | Open / descend | `BrowserDescend` → check `Browser::descend()`; if Dir → `App::enter_dir()`; if Markdown → `App::open_from_browser()` → Reader mode |
| `Backspace` | Ascend to parent | `BrowserAscend` → `App::ascend()` → `Browser::new(parent_dir)` |
| `q` / `Ctrl-C` | Quit | `Quit` |

### Mouse — Browser Mode

| Gesture | Action | What happens internally |
|---|---|---|
| Scroll wheel | Scroll listing | `BrowserScroll(delta)` → clamp `browser.scroll` to available entries |
| Click entry | Select + open/descend | `BrowserClickAt{row}` → compute `scroll + row_offset`; if in-range set `selected`; then descend/open |

Clicks below the last entry are ignored (no out-of-range selection).

---

## Colour and Theme

**Bella is dark-only today.** `app.rs` and `render_worker.rs` construct `Theme::dark()` directly at
every render site; there is no code path that selects a different palette. `Theme::dark()` carries
bastiel's "cool-aurora" brand palette (`business/bastiel/src/app/globals.css` in the company-brain
repo — primary blue, sky-blue and purple accents on a near-black ground), ported 1:1 so bella's
rendering matches the wider Bastion ecosystem's flagship web app rather than carrying its own
unrelated scheme. It replaced the previous Catppuccin Mocha palette.

The pieces of a theming system exist in `bella-engine` but have **no call site in the binary** —
verified by grepping every `.rs` file in the workspace:

| Item | Where | State |
|---|---|---|
| `theme::resolve(name, cfg)` | `theme.rs` | Maps `"light"` / `"dark"` / anything-else to a `Theme`. Never called. |
| `detect_terminal_theme()` | `theme.rs` | Reads `COLORFGBG` (bg 7–15 → light, 0–6 → dark). Only reachable through `resolve`, so never called. |
| `Theme::light()` | `theme.rs` | Catppuccin Latte — unrelated to bastiel; untouched by the cool-aurora change since it has no call site. Only reachable through `resolve`. |
| `Theme::mission_control()` | `theme.rs` | A third palette (indigo/violet/cyan, coordinated with bastion's console) **no name maps to** — `resolve` cannot return it. |
| `md_config::load()` | `md_config.rs` | Reads `theme`, `width`, `line_numbers` from `<config-dir>/md/config.toml`. Never called, so the file is never read. |

**The status line does read the active theme.** `App.theme` (set from `Theme::dark()` in both
constructors) drives `ui::draw_statusline`/`draw_browser_statusline`'s `fg`/`bg` via
`theme.status_fg`/`status_bg` — previously these two functions hardcoded `Color::Black`/`Color::White`
and ignored the theme entirely, so the status bar never reflected any palette, past or present. Body
content (headings, links, code, quotes, rules) is still themed per-render, via the `Theme` passed to
`request_render`/`render_with_edit`, not through `App.theme` — that field exists only so always-on
chrome the render worker doesn't own has something to read.

Two consequences worth stating plainly:

- **The config file does nothing.** Its path is `dirs::config_dir()`-relative, so it would be
  `~/Library/Application Support/md/config.toml` on macOS and `~/.config/md/config.toml` on Linux —
  but neither is opened.
- **Colour *depth* detection is real and does work.** That is a separate mechanism in
  [`palette.rs`](../crates/bella-engine/src/palette.rs), probing `COLORTERM` → `TERM_PROGRAM` →
  `TERM` and downgrading RGB to the nearest xterm-256 entry. It runs on every render.

Wiring this up is tracked as a limitation in the README's roadmap section.

## Text Selection and Clipboard

Drag-select and double-click word-select both copy to the system clipboard via the `arboard` crate. If the clipboard write fails (e.g. no display server in a headless environment), the error is shown in the status line for one frame and then cleared.

Selection is visual-only — it does not modify the source file. The selection is preserved on screen after copying so you can see what was captured; it clears on the next render (scroll, new file, terminal resize).
