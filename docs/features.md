---
type: Reference
title: Feature Reference
description: All keybindings and mouse gestures with descriptions of what happens internally.
---

# Feature Reference

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

Bella uses **Catppuccin** colour themes. Auto-detection reads `COLORFGBG`: bg value 7–15 → Latte (light), 0–6 → Mocha (dark). The terminal emulator's colour depth is detected separately via `COLORTERM`/`TERM_PROGRAM`/`TERM` environment probes; RGB colours are downgraded to the nearest xterm-256 entry on terminals that don't support truecolor.

An optional config file at `~/.config/md/config.toml` can override the theme and terminal width:

```toml
theme = "dark"   # or "light"
width = 100
```

---

## Text Selection and Clipboard

Drag-select and double-click word-select both copy to the system clipboard via the `arboard` crate. If the clipboard write fails (e.g. no display server in a headless environment), the error is shown in the status line for one frame and then cleared.

Selection is visual-only — it does not modify the source file. The selection is preserved on screen after copying so you can see what was captured; it clears on the next render (scroll, new file, terminal resize).
