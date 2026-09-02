//! Parity + behavior test for the background render worker
//! (`bella::render_worker::RenderWorker`).
//!
//! Drives the worker directly rather than through the TUI event loop, since
//! raw-mode loop responsiveness itself isn't unit-testable. Covers:
//! - parity: async `Ready` output == synchronous `bella-engine` render output,
//!   for a representative document and a large (multi-thousand-line) input.
//! - Loading-first: a render result is not available immediately after the
//!   request is made (the worker never blocks the caller).
//! - staleness: after two rapid requests, the final delivered `Ready`
//!   corresponds to the latest request, not a stale earlier one.
//! - edge cases: empty document; a render requested while a previous one is
//!   still in flight.

use std::time::{Duration, Instant};

use bella::render_worker::{RenderWorker, is_latest};
use bella_engine::links::TableExpansions;
use bella_engine::markdown::render_with_edit;
use bella_engine::theme::Theme;

/// Poll `try_recv_latest` until a result is available or `timeout` elapses.
fn wait_for_result(
    worker: &mut RenderWorker,
    timeout: Duration,
) -> Option<bella::render_worker::RenderResult> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(r) = worker.try_recv_latest() {
            return Some(r);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// A representative markdown document exercising headings, lists, links,
/// code, and a table.
fn representative_doc() -> String {
    r#"# Title

Some **bold** and _italic_ text with a [link](https://example.com).

- item one
- item two
  - nested item

```rust
fn main() {
    println!("hello");
}
```

| a | b |
|---|---|
| 1 | 2 |

> a blockquote

- [ ] todo item
- [x] done item
"#
    .to_string()
}

/// A large, multi-thousand-line markdown document.
fn large_doc(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 24);
    for i in 0..lines {
        s.push_str(&format!("- line item number {i}\n"));
    }
    s
}

#[test]
fn parity_representative_doc() {
    let source = representative_doc();
    let width = 80;
    let theme = Theme::dark();

    let expected = render_with_edit(&source, None, width, &theme, None, &TableExpansions::new());

    let mut worker = RenderWorker::spawn();
    let generation =
        worker.request_render(source, None, width, theme, None, TableExpansions::new());
    let result = wait_for_result(&mut worker, Duration::from_secs(5))
        .expect("worker must eventually deliver a render result");
    assert_eq!(result.generation, generation);
    assert_eq!(
        result.rendered.lines, expected.lines,
        "async render output must be line-for-line equal to the synchronous render"
    );
}

#[test]
fn parity_large_doc() {
    let source = large_doc(5_000);
    let width = 100;
    let theme = Theme::dark();

    let expected = render_with_edit(&source, None, width, &theme, None, &TableExpansions::new());

    let mut worker = RenderWorker::spawn();
    worker.request_render(source, None, width, theme, None, TableExpansions::new());
    let result = wait_for_result(&mut worker, Duration::from_secs(15))
        .expect("worker must eventually deliver a render result for a large document");
    assert_eq!(
        result.rendered.lines, expected.lines,
        "async render output must match synchronous render for a large (multi-thousand-line) document"
    );
}

#[test]
fn parity_empty_document() {
    let source = String::new();
    let width = 80;
    let theme = Theme::dark();

    let expected = render_with_edit(&source, None, width, &theme, None, &TableExpansions::new());

    let mut worker = RenderWorker::spawn();
    worker.request_render(source, None, width, theme, None, TableExpansions::new());
    let result = wait_for_result(&mut worker, Duration::from_secs(5))
        .expect("worker must deliver a result for an empty document");
    assert_eq!(result.rendered.lines, expected.lines);
}

#[test]
fn loading_precedes_ready() {
    // A large document gives the worker enough work that the result is very
    // unlikely to already be sitting in the channel by the time we check —
    // demonstrating the request itself never blocks the caller.
    let source = large_doc(20_000);
    let mut worker = RenderWorker::spawn();
    worker.request_render(
        source,
        None,
        80,
        Theme::dark(),
        None,
        TableExpansions::new(),
    );

    // Immediately after requesting, no result should be ready yet — the
    // caller is still in a "Loading" state.
    assert!(
        worker.try_recv_latest().is_none(),
        "a render result must not be available immediately after requesting \
         (the worker must not block the caller)"
    );

    // But it must eventually arrive.
    let result = wait_for_result(&mut worker, Duration::from_secs(15));
    assert!(
        result.is_some(),
        "the worker must eventually deliver a Ready result"
    );
}

#[test]
fn stale_render_is_discarded_in_favor_of_latest() {
    let mut worker = RenderWorker::spawn();
    let _g1 = worker.request_render(
        "# first request".to_string(),
        None,
        80,
        Theme::dark(),
        None,
        TableExpansions::new(),
    );
    let g2 = worker.request_render(
        "# second request, in flight while the first was still pending".to_string(),
        None,
        80,
        Theme::dark(),
        None,
        TableExpansions::new(),
    );

    // Give the worker time to process both requests so both results are
    // likely buffered by the time we drain.
    std::thread::sleep(Duration::from_millis(50));

    let mut latest = None;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(r) = worker.try_recv_latest() {
            latest = Some(r);
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let latest = latest.expect("expected a render result");

    assert_eq!(
        latest.generation, g2,
        "the final delivered result must correspond to the latest of two rapid requests"
    );
    assert!(is_latest(&latest, worker.latest_requested_generation()));
}

#[test]
fn render_requested_while_previous_still_in_flight() {
    let mut worker = RenderWorker::spawn();

    // Kick off a large render that will still be in flight...
    worker.request_render(
        large_doc(10_000),
        None,
        80,
        Theme::dark(),
        None,
        TableExpansions::new(),
    );

    // ...then immediately request another render before the first has
    // necessarily completed.
    let source2 = representative_doc();
    let width2 = 80;
    let theme2 = Theme::dark();
    let expected2 = render_with_edit(
        &source2,
        None,
        width2,
        &theme2,
        None,
        &TableExpansions::new(),
    );
    let g2 = worker.request_render(source2, None, width2, theme2, None, TableExpansions::new());

    // The worker must not deadlock or drop the second request: eventually a
    // result for the latest generation must arrive.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut final_result = None;
    loop {
        if let Some(r) = worker.try_recv_latest()
            && r.generation == g2
        {
            final_result = Some(r);
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    let final_result =
        final_result.expect("worker must eventually deliver the result for the second request");
    assert_eq!(final_result.rendered.lines, expected2.lines);
}
