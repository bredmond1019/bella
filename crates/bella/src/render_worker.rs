//! Background render worker: offloads `bella-engine` markdown parsing/rendering
//! onto a dedicated `std::thread` so the TUI event loop never blocks on a
//! synchronous render.
//!
//! The worker is driven by a request/response pair of `mpsc` channels. Every
//! request carries a monotonically increasing *generation* token; the caller
//! uses the token on delivered results to discard stale renders (e.g. a
//! render kicked off for a since-superseded width or file).
//!
//! This module owns no TUI state and does not touch `crates/bella-engine/` —
//! it only calls the engine's existing public render API from a background
//! thread.
//!
//! Not yet wired into `app.rs`/`events.rs` (that happens in a follow-up
//! task), so its public API is allowed to look unused for now.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvError, Sender, TryRecvError};

use bella_engine::markdown::{EditCtx, Rendered, render_with_edit};
use bella_engine::theme::Theme;

use bella_engine::links::TableExpansions;

/// A single render request sent to the worker thread.
struct RenderRequest {
    generation: u64,
    source: String,
    base_dir: Option<PathBuf>,
    width: u16,
    theme: Theme,
    edit: Option<EditCtx>,
    tables: TableExpansions,
}

/// A render result delivered back from the worker thread, tagged with the
/// generation of the request that produced it.
pub struct RenderResult {
    pub generation: u64,
    pub rendered: Rendered,
}

/// Owns the channels to a background render thread.
///
/// Dropping the worker drops the request sender, which causes the background
/// thread's `recv()` loop to observe a disconnected channel and exit cleanly.
pub struct RenderWorker {
    request_tx: Sender<RenderRequest>,
    result_rx: Receiver<RenderResult>,
    next_generation: u64,
    /// Highest generation actually requested so far. Used so callers can
    /// tell whether a drained result is the latest.
    latest_requested: u64,
    _handle: std::thread::JoinHandle<()>,
}

impl RenderWorker {
    /// Spawn the background render thread and return a handle to it.
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<RenderRequest>();
        let (result_tx, result_rx) = mpsc::channel::<RenderResult>();

        let handle = std::thread::Builder::new()
            .name("bella-render-worker".to_string())
            .spawn(move || worker_loop(request_rx, result_tx))
            .expect("spawn render worker thread");

        Self {
            request_tx,
            result_rx,
            next_generation: 0,
            latest_requested: 0,
            _handle: handle,
        }
    }

    /// Request a render for `source` at `width`. Returns the generation token
    /// assigned to this request; never blocks on the render itself.
    ///
    /// If the worker thread has terminated (e.g. panicked), the request is
    /// silently dropped and the returned generation will simply never
    /// produce a result — callers should treat "no Ready yet" as normal.
    pub fn request_render(
        &mut self,
        source: String,
        base_dir: Option<PathBuf>,
        width: u16,
        theme: Theme,
        edit: Option<EditCtx>,
        tables: TableExpansions,
    ) -> u64 {
        self.next_generation += 1;
        let generation = self.next_generation;
        self.latest_requested = generation;

        // Ignore send errors: a disconnected worker means the render will
        // never arrive, which the polling caller already handles by staying
        // in `Loading`.
        let _ = self.request_tx.send(RenderRequest {
            generation,
            source,
            base_dir,
            width,
            theme,
            edit,
            tables,
        });

        generation
    }

    /// Non-blocking drain of any results currently buffered from the worker.
    /// Returns only the result with the highest generation among those
    /// available (older, superseded results are discarded), or `None` if no
    /// result is ready yet.
    pub fn try_recv_latest(&mut self) -> Option<RenderResult> {
        let mut latest: Option<RenderResult> = None;
        loop {
            match self.result_rx.try_recv() {
                Ok(result) => {
                    if latest
                        .as_ref()
                        .is_none_or(|l| result.generation > l.generation)
                    {
                        latest = Some(result);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        latest
    }

    /// Blocking receive of the next single result (used by tests that need
    /// deterministic waiting rather than polling in a spin loop).
    pub fn recv_blocking(&self) -> Result<RenderResult, RecvError> {
        self.result_rx.recv()
    }

    /// The generation token of the most recent render actually requested.
    /// Callers use this to decide whether a drained `RenderResult` is stale.
    pub fn latest_requested_generation(&self) -> u64 {
        self.latest_requested
    }
}

/// Whether `result` corresponds to the latest known request generation,
/// i.e. is not stale.
pub fn is_latest(result: &RenderResult, latest_requested_generation: u64) -> bool {
    result.generation == latest_requested_generation
}

/// The background thread body: receive requests, render synchronously
/// (off the caller's thread), and send results back tagged with their
/// generation. Exits cleanly when the request channel disconnects.
fn worker_loop(request_rx: Receiver<RenderRequest>, result_tx: Sender<RenderResult>) {
    while let Ok(req) = request_rx.recv() {
        let rendered = render_with_edit(
            &req.source,
            req.base_dir.as_deref(),
            req.width,
            &req.theme,
            req.edit,
            &req.tables,
        );
        // If the receiver has been dropped (caller gone), stop the loop.
        if result_tx
            .send(RenderResult {
                generation: req.generation,
                rendered,
            })
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use bella_engine::links::TableExpansions;
    use bella_engine::theme::Theme;

    use super::*;

    fn wait_for_result(worker: &mut RenderWorker, timeout: Duration) -> Option<RenderResult> {
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

    #[test]
    fn request_render_returns_increasing_generations() {
        let mut worker = RenderWorker::spawn();
        let g1 = worker.request_render(
            "# a".to_string(),
            None,
            80,
            Theme::dark(),
            None,
            TableExpansions::new(),
        );
        let g2 = worker.request_render(
            "# b".to_string(),
            None,
            80,
            Theme::dark(),
            None,
            TableExpansions::new(),
        );
        assert!(g2 > g1, "generation tokens must increase per request");
    }

    #[test]
    fn worker_delivers_a_render_result() {
        let mut worker = RenderWorker::spawn();
        let generation = worker.request_render(
            "# Hello".to_string(),
            None,
            80,
            Theme::dark(),
            None,
            TableExpansions::new(),
        );

        let result =
            wait_for_result(&mut worker, Duration::from_secs(5)).expect("expected a render result");
        assert_eq!(result.generation, generation);
        assert!(!result.rendered.lines.is_empty());
    }

    #[test]
    fn try_recv_latest_discards_stale_results() {
        let mut worker = RenderWorker::spawn();
        let _g1 = worker.request_render(
            "# a".to_string(),
            None,
            80,
            Theme::dark(),
            None,
            TableExpansions::new(),
        );
        let g2 = worker.request_render(
            "# b".to_string(),
            None,
            80,
            Theme::dark(),
            None,
            TableExpansions::new(),
        );

        // Give the worker time to process both requests before draining, so
        // both results are likely buffered when we call try_recv_latest.
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
            "try_recv_latest must return the highest-generation result, discarding stale ones"
        );
        assert!(is_latest(&latest, worker.latest_requested_generation()));
    }
}
