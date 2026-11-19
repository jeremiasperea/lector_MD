# mdv — Markdown viewer

A small Markdown reader written in Rust, with two front ends that share one
rendering core:

- **`mdv`** — a terminal viewer with a built-in pager and syntax-highlighted code blocks.
- **`mdv-desktop`** — a [Tauri 2](https://tauri.app) desktop app that shows the
  document as sanitized HTML.

Both treat every document as **untrusted input**: what you open can format text,
but it cannot run code, read your files, or take over your terminal.

## Features

| | Terminal (`mdv`) | Desktop (`mdv-desktop`) |
|---|---|---|
| GitHub-flavored Markdown (tables, task lists, strikethrough, footnotes) | ✓ | ✓ |
| Syntax highlighting for fenced code | ✓ (truecolor) | ✓ (`base16-ocean.dark`) |
| Pager with `less`-style keys | ✓ | — |
| External links open in the default browser / mail client | — | ✓ |
| Reading width / full-width toggle, remembered across launches | — | ✓ |
| Hardened against hostile documents | control characters stripped | HTML sanitized, file reads and links restricted |

## Project layout

```
crates/
├── mdv-core/      Markdown → sanitized HTML (pulldown-cmark, syntect, ammonia)
├── mdv/           Terminal viewer (clap, termimad, syntect)
└── mdv-desktop/   Tauri app: Rust commands in src/, vanilla HTML/CSS/JS in ui/
```

## Requirements

- Rust **1.85 or newer** (the crates use edition 2024).
- For the desktop app: the [Tauri system prerequisites](https://tauri.app/start/prerequisites/)
  for your OS (on Linux, WebKitGTK and friends).
- Optional: [`tauri-cli`](https://tauri.app/reference/cli/) to build installers
  (`cargo install tauri-cli --version "^2"`).
- Optional: Node.js 18+ to run the desktop UI tests.

## Terminal viewer

```sh
cargo run -p mdv -- README.md            # open in the pager
cargo run -p mdv -- README.md --no-pager # print the whole document
cargo install --path crates/mdv          # install the `mdv` binary
```

When standard output is not a terminal (for example, piped into another
command), `mdv` prints the document instead of opening the pager.

### Pager keys

| Key | Action |
|---|---|
| `j`, `↓`, `Enter` | Down one line |
| `k`, `↑` | Up one line |
| `Space`, `f`, `PageDown` | Down one page |
| `b`, `PageUp` | Up one page |
| `Ctrl+d` / `Ctrl+u` | Down / up half a page |
| `g`, `Home` / `G`, `End` | Top / bottom |
| `q`, `Esc`, `Ctrl+c` | Quit |

## Desktop app

```sh
cargo run -p mdv-desktop                  # start and pick a file with "Open file"
cargo run -p mdv-desktop -- notes.md      # open a file directly
```

To build installers for your platform:

```sh
cd crates/mdv-desktop
cargo tauri build
```

Links inside the document behave like this:

- `#anchor` links scroll within the page.
- `http://`, `https://` and `mailto:` links open in your default browser or mail client.
- Anything else (relative paths, `file:`, custom schemes) is refused, and the
  reason is shown in the header.

## Security model

Opening a Markdown file should be as safe as reading a text file. The
safeguards, and where they live:

- **Sanitized HTML** (`mdv-core`): scripts, event handlers and `javascript:`
  links are removed; the only form element allowed is a task-list checkbox.
  A strict Content Security Policy in `tauri.conf.json` backs this up.
- **File reads** (`mdv-desktop`): the `open_file` command only reads files the
  user chose, through the file picker or the command line. Script that slipped
  past the sanitizer still could not read arbitrary files.
- **Links** (`mdv-desktop`): only browser and mail links leave the app. Mail
  links keep just the address; any prefilled `cc`, `bcc`, subject or body is
  dropped, and a mail link without an address is refused.
- **Terminal output** (`mdv`): control characters other than newlines and tabs
  are stripped before rendering, so a document cannot clear the screen, write
  to your clipboard, fake links or otherwise drive your terminal.
- **Permissions**: the Tauri window only has the `core:default` capability.

## Development

```sh
cargo test --workspace                                # Rust tests
node --test crates/mdv-desktop/ui-tests/app.test.mjs  # desktop UI tests (real app.js, stubbed DOM)
cargo run -p mdv-core --example syntax_css > crates/mdv-desktop/ui/syntax.css  # regenerate highlight CSS
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org/).
