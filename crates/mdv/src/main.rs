mod pager;
mod render;

use std::{
    fs,
    io::{IsTerminal, stdout},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::Parser;
use render::Renderer;
use termimad::{MadSkin, terminal_size};

/// Render Markdown files in the terminal.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Markdown file to render
    file: PathBuf,

    /// Print the whole document instead of opening the pager
    #[arg(long)]
    no_pager: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let content = fs::read_to_string(&args.file)
        .with_context(|| format!("cannot read {}", args.file.display()))?;

    let renderer = Renderer::new(MadSkin::default());
    // A pager only makes sense on an interactive terminal; when piped, print plainly.
    if args.no_pager || !stdout().is_terminal() {
        let width = terminal_size().0.into();
        for line in renderer.render(&content, width) {
            println!("{line}");
        }
        return Ok(());
    }
    pager::run(&content, &renderer)
}
