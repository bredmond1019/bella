
Bella

A terminal markdown viewer with full mouse support — scroll, click links, drag-select text and copy to your
system clipboard, all from inside a  ratatui  TUI. Local-only: it reads files off disk and opens a system
browser tab for external links; nothing else talks to the network.

│ Part of the Bastion ecosystem — see the bastion-os front door for the wider architecture. Bella itself works
│ standalone; nothing below requires it.

What this is for

You want to read (or browse a folder of) markdown files without leaving the terminal — with proper
link-following, in-document search, and mouse selection, instead of  less / cat -ing raw markdown source.
Point it at a file, or at nothing, and it opens a file browser.

Quickstart

Run these in a shell:

  # 1. Clone and build
  git clone https://github.com/bredmond1019/bella && cd bella
  cargo build --release

  # 2. Open a specific file
  cargo run --release -p bella -- README.md

  # 3. Or browse a directory (defaults to the current directory if you omit the argument)
  cargo run --release -p bella
  cargo run --release -p bella -- some/dir

Press  q  or  Ctrl-C  to quit from either mode.

To install the  bella  binary onto your  PATH  instead of running it via  cargo run :

  cargo install --path crates/bella

Prerequisites

┌────────────────────────────────────┬─────────────────────────────────┬─────────────────────────────────────┐
│ Requirement                        │ Why                             │ If missing                          │
├────────────────────────────────────┼─────────────────────────────────┼─────────────────────────────────────┤
 bella · README.md · 42/191  j/k scroll · / search · [ ] history · q quit
