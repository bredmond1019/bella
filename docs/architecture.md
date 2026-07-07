---
type: Architecture
title: Bella Architecture
description: Two-crate design, render pipeline, async render worker, event loop, coordinate system, and mode model.
doc_id: architecture
layer: [console]
project: bella
status: active
keywords: [two-crate architecture, bella-engine, render pipeline, async render worker, event loop, coordinate system]
related: [modules, D2-engine-app-crate-split]
---

# Architecture — Bella

## Two-Crate Split

Bella is a Cargo workspace with two crates. The boundary is intentional — it is the legal and narrative boundary, not just a code-organisation choice.

| Crate | Role | Origin |
|---|---|---|
| `bella-engine` | Render/layout library | MIT derivative of `zemse/hackmd @ 7650cdc` — attributed, progressively rewritten |
| `bella` | TUI binary | Original work |

The engine crate contains everything that touches markdown: parsing, word-wrapping, link resolution, syntax highlighting, checkbox/table geometry, and coordinate conversion. It has **zero I/O and zero terminal dependencies** — every function takes explicit parameters and returns plain data. The app crate contains everything that touches the terminal: the event loop, the App state machine, and the ratatui draw calls.

See `planning/decisions/D2-engine-app-crate-split.md` for the full rationale.

## Render Pipeline

Markdown source text flows through `bella-engine` in three stages:

```
source: &str
  │
  ▼  pulldown-cmark (TABLES, STRIKETHROUGH, TASKLISTS, FOOTNOTES, SMART_PUNCTUATION, WIKILINKS)
Event stream
  │
  ▼  Block tree construction
   headings, paragraphs, code fences, blockquotes, lists, tables, hrules, …
  │
  ▼  Layout pass (word-wrap to terminal width)
   Vec<Line<'static>>  ← ratatui text cells, styled with Catppuccin theme colours
   LinkMap             ← hyperlink spans with (line, col_start, col_end, target)
   CheckboxMap         ← task-list item spans
   TableMap            ← table click-geometry (column x-ranges, row y-ranges)
   …
  │
  ▼  Rendered { lines, link_map, checkbox_map, table_map, images, headings, … }
```

`bella_engine::render_with_edit(source, base_dir, width, theme, edit, tables)` is the pipeline's single entry point, but the app never calls it inline on the event-loop thread. Instead it runs on a background worker — see **Async Render Worker** below — and the result is applied into `App` once it lands. Once applied, the result is stored in `App` and never mutated — all interactive overlays (search highlights, link focus, selection) are applied at draw time by `ui::draw_reader`.

## Async Render Worker

Large documents can take long enough to parse/layout that running `render_with_edit` inline would stall the terminal — freezing keys (notably `q` to quit) until it finished. `crates/bella/src/render_worker.rs` moves that work off the event-loop thread:

```
App::new / App::render(width) / App::load_file(path)
  │
  ▼  render_worker.request_render(source, base_dir, width, theme, …) → generation: u64
  │  (returns immediately — does not block)
  │
  App.render_state = Loading            RenderWorker's background std::thread
  App.lines = "Loading…" placeholder      │
                                           ▼  bella_engine::render_with_edit(…)
                                          Rendered { lines, link_map, checkbox_map, headings, … }
                                           │
                                           ▼  sent back over an mpsc channel, tagged with `generation`
  │
  ▼  App::poll_render() — called every run_loop tick
     try_recv_latest() → highest-generation buffered result (older ones discarded)
     if result.generation == App.render_generation:
         apply it, App.render_state = Ready
     else:
         stale (superseded by a later resize/load) — discard, keep polling
```

Every `request_render` call returns a strictly increasing generation token. `App` stores the latest one it requested (`render_generation`); a result is only applied if its tag matches — anything older was superseded by a subsequent resize or file load and is silently dropped. This means a slow render for a since-abandoned width/file can never clobber a newer one that finished first. The worker thread processes requests FIFO with no preemption, so a very large render in flight can delay a smaller subsequent one's `Ready` state, but never its correctness.

`App::block_until_ready()` (test-only) blocks on the same channel to give the ~190 pre-existing synchronous tests deterministic `Ready` state without rewriting them around a poll loop.

## Event Loop

```
terminal.draw(|frame| ui::draw_reader(frame, …))
  │
  ▼  app.poll_render()                        ← drain any render result that landed
  │
  ▼  crossterm::event::poll(50ms timeout)
  │
  ├─ timed out, no event → loop back to draw (picks up the render just applied)
  │
  ├─ KeyEvent   → map_key / map_browser_key / map_search_key  →  Action
  ├─ MouseEvent → map_mouse / map_browser_mouse               →  Action
  └─ ResizeEvent → recompute viewport_height + app.render(width) (kicks off an async re-render)
  │
  ▼  events::apply(action, &mut app)   ← pure state mutation
  │
  ▼  loop
```

The loop polls with a timeout (`events::EVENT_POLL_TIMEOUT`, 50ms) instead of blocking on `crossterm::event::read()`, and drains the render worker each tick — so a markdown parse in flight never stalls input handling. All three key/mouse mapper functions are **pure** (no terminal I/O, no App mutation) so they are unit-tested without a real terminal. `apply` takes `&mut App` and dispatches to the appropriate App method. The draw call comes first each iteration so the initial render (or the `Loading…` placeholder) is visible before waiting for input.

## Coordinate System

Terminal positions use `(col, row)` — 0-indexed, top-left origin. The render area is split at draw time:

```
┌─────────────────────────────┐  ← row 0
│                             │
│   body_area (Rect)          │  ← markdown content lines, scrolled
│                             │
├─────────────────────────────┤  ← row viewport_height
│   status line (1 row)       │
└─────────────────────────────┘
```

`draw_reader` stores the `body_area` Rect in `App` after each frame so mouse events can convert screen coordinates to content positions.

The conversion is:

```
body_pos(viewport, line_numbers, line_count, scroll, col, row)
  → Option<(content_row_index, local_col)>
```

`content_row_index = scroll + (row - body_area.y)`. Out-of-bounds returns `None` — no panics on clicks outside the content area.

`select_word_at` extends `body_pos` by walking left/right along the display string (unicode-width aware) to find the word under the cursor.

Browser mode stores `browser_area` analogously for click hit-testing against the directory listing.

## Mode Model

`App` carries a `Mode` enum:

```rust
enum Mode { Reader, Browser }
```

`run_loop` gates event dispatch on `app.mode`:

- **Reader** — `map_key` / `map_mouse` — full reader/search/selection/history bindings.
- **Browser** — `map_browser_key` / `map_browser_mouse` — cursor navigation + open/descend/ascend.

Search has its own sub-mode, tracked by `app.search.input_mode: bool`, which redirects key events to `map_search_key` before the normal reader mapper.

Transitions:
- `bella <dir>` or `bella` (no args) → Browser mode at startup
- `bella <file>` → Reader mode at startup
- Browser `Enter` on a `.md` file → `App::open_from_browser()` → Reader mode; saves `browser_origin`
- Reader `Backspace` → `App::back_to_browser()` → Browser mode; restores saved dir + cursor
- Browser `Enter` on a dir → `App::enter_dir()` → stays in Browser mode
- Browser `Backspace` / `..` entry → `App::ascend()` → stays in Browser mode
