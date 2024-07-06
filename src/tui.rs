use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use miette::Result;
use ratatui::{prelude::*, widgets::*};
use std::{
    io::{self, Stdout},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::AppState;

pub fn setup_terminal(
) -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(
        stdout,
    ))?)
}

pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    Ok(terminal.show_cursor()?)
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: Arc<Mutex<AppState>>,
) -> Result<(), io::Error> {
    Ok(loop {
        terminal.draw(|f| ui(f, state.clone()))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if KeyCode::Char('q') == key.code {
                    break;
                }
            }
        }
    })
}

fn ui<B: Backend>(
    f: &mut Frame<B>,
    state: Arc<Mutex<AppState>>,
) {
    let s = state.lock().unwrap();
    let dbs: Vec<u64> = s
        .decibels
        .iter()
        .rev()
        .take(300)
        .map(|db| db.abs() as u64)
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                // Constraint::Length(3),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(f.size());
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title("decibels")
                .borders(Borders::LEFT | Borders::RIGHT),
        )
        .data(&dbs)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(sparkline, chunks[0]);
}
