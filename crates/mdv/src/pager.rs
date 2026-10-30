//! Interactive, less-like pager over pre-rendered lines.

use std::io::{Write, stdout};

use anyhow::Result;
use termimad::crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{
        self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use crate::render::Renderer;

/// Restores the terminal on drop, even when an error or panic unwinds.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        // No wrapping: long code lines are clipped instead of spilling into the next row.
        execute!(
            stdout(),
            EnterAlternateScreen,
            DisableLineWrap,
            cursor::Hide
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show, EnableLineWrap, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

enum Action {
    Quit,
    Lines(isize),
    Pages(isize),
    HalfPages(isize),
    Top,
    Bottom,
    None,
}

fn action_for(key: KeyEvent) -> Action {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('d') if ctrl => Action::HalfPages(1),
        KeyCode::Char('u') if ctrl => Action::HalfPages(-1),
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => Action::Lines(1),
        KeyCode::Char('k') | KeyCode::Up => Action::Lines(-1),
        KeyCode::Char(' ') | KeyCode::Char('f') | KeyCode::PageDown => Action::Pages(1),
        KeyCode::Char('b') | KeyCode::PageUp => Action::Pages(-1),
        KeyCode::Char('g') | KeyCode::Home => Action::Top,
        KeyCode::Char('G') | KeyCode::End => Action::Bottom,
        _ => Action::None,
    }
}

struct Pager<'a> {
    markdown: &'a str,
    renderer: &'a Renderer,
    lines: Vec<String>,
    scroll: usize,
    height: usize,
}

impl Pager<'_> {
    fn layout(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        self.height = usize::from(height);
        self.lines = self.renderer.render(self.markdown, usize::from(width));
        self.scroll_by(0);
        Ok(())
    }

    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.height)
    }

    fn scroll_by(&mut self, delta: isize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(delta)
            .min(self.max_scroll());
    }

    fn draw(&self) -> Result<()> {
        let mut out = stdout().lock();
        for row in 0..self.height {
            let line = self.lines.get(self.scroll + row).map_or("", String::as_str);
            queue!(
                out,
                cursor::MoveTo(0, row as u16),
                Clear(ClearType::CurrentLine),
                Print(line)
            )?;
        }
        out.flush()?;
        Ok(())
    }
}

pub fn run(markdown: &str, renderer: &Renderer) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut pager = Pager {
        markdown,
        renderer,
        lines: Vec::new(),
        scroll: 0,
        height: 0,
    };
    pager.layout()?;

    loop {
        pager.draw()?;
        let page = pager.height as isize;
        match event::read()? {
            Event::Key(key) => match action_for(key) {
                Action::Quit => break,
                Action::Lines(n) => pager.scroll_by(n),
                Action::Pages(n) => pager.scroll_by(n * page),
                Action::HalfPages(n) => pager.scroll_by(n * page / 2),
                Action::Top => pager.scroll = 0,
                Action::Bottom => pager.scroll = pager.max_scroll(),
                Action::None => {}
            },
            Event::Resize(..) => pager.layout()?,
            _ => {}
        }
    }
    Ok(())
}
