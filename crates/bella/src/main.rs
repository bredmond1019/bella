//! bella — terminal markdown viewer entry point.
//!
//! Parses CLI args, reads the file, sets up the ratatui terminal, then
//! hands off to the event loop.  `main.rs` itself stays thin; logic lives
//! in `app`, `ui`, and `events`.

use std::path::PathBuf;

use bella::{app, events};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

/// bella — beautiful terminal markdown viewer.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Markdown file or directory to open.  Omit to browse the current directory.
    file: Option<PathBuf>,

    /// Print this binary's build provenance (git_sha, dirty, source_dir) as one JSON
    /// line to stdout and exit immediately — before any terminal setup runs. This is
    /// the cross-binary contract `toolchain-freshness` uses to query registered corpus
    /// writers (mirrors `mev --build-stamp` / `bastion --build-stamp`); do not add,
    /// rename, or drop a key from the emitted shape.
    #[arg(long)]
    build_stamp: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle --build-stamp before any terminal setup: enable_raw_mode() +
    // EnterAlternateScreen below would swallow this output into the alternate screen,
    // wiped on restore with the exit code still reading success. Machine consumers
    // (mev's toolchain-freshness check) need this on stdout, undisturbed.
    if cli.build_stamp {
        println!(
            "{}",
            serde_json::to_string(&bella::buildstamp::stamp_json())?
        );
        return Ok(());
    }

    // --- terminal setup ---
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("enter alternate screen + enable mouse")?;

    // Panic hook: restore terminal before the default handler prints.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("create terminal")?;

    let result = run(&mut terminal, cli.file);

    // --- always restore ---
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    file: Option<PathBuf>,
) -> Result<()> {
    let size = terminal.size().context("get terminal size")?;

    let mut app = match file {
        // No argument → browser at the current working directory.
        None => {
            let cwd = std::env::current_dir().context("get current directory")?;
            app::App::new_browser(cwd, size.width, size.height)
        }
        // Directory argument → browser rooted at that directory.
        Some(path) if path.is_dir() => app::App::new_browser(path, size.width, size.height),
        // File argument → reader (existing behaviour).
        Some(path) => {
            let src = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read {:?}", path))?;
            app::App::new(src, path, size.width, size.height)
        }
    };

    // Both constructors default to Theme::dark(); resolve the real theme once
    // here — "auto" checks ~/.config/md/config.toml's `theme` field first,
    // then falls back to $COLORFGBG terminal detection (bella-engine's
    // theme::resolve/md_config::load, dormant since BE.2.F was parked
    // wontfix; revived per OP.revive-theming-from-wontfix-be-2-f, D5).
    let cfg = bella_engine::md_config::load();
    let theme = bella_engine::theme::resolve("auto", &cfg);
    app.set_theme(theme);

    events::run_loop(terminal, app)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Cli;

    #[test]
    fn file_arg_parses() {
        let m = Cli::try_parse_from(["bella", "some.md"]).expect("should parse");
        assert_eq!(m.file.as_ref().unwrap().to_str().unwrap(), "some.md");
    }

    #[test]
    fn no_arg_parses_to_none() {
        let m = Cli::try_parse_from(["bella"]).expect("no arg must parse successfully");
        assert!(m.file.is_none(), "missing file arg must parse to None");
    }

    #[test]
    fn build_stamp_flag_parses_standalone() {
        let m = Cli::try_parse_from(["bella", "--build-stamp"])
            .expect("--build-stamp must parse with no other argument");
        assert!(m.build_stamp, "--build-stamp must set build_stamp true");
        assert!(
            m.file.is_none(),
            "--build-stamp alone must leave file as None"
        );
    }

    #[test]
    fn command_compiles() {
        // Ensures the clap definition is self-consistent.
        Cli::command().debug_assert();
    }
}
