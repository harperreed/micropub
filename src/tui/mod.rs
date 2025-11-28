// ABOUTME: Terminal User Interface (TUI) module for micropub
// ABOUTME: Provides interactive interface for managing drafts, posts, and media

mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

pub use app::App;

/// Run the TUI application
pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new().await?;

    // Run the app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

/// Main event loop
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => {
                        if app.confirm_quit() {
                            return Ok(());
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => app.next_item(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous_item(),
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Enter => app.select_item().await?,
                    KeyCode::Char('y') if app.awaiting_confirmation() => {
                        app.confirm_action().await?;
                    }
                    KeyCode::Char('n') if app.awaiting_confirmation() => {
                        app.cancel_action();
                    }
                    KeyCode::Char('p') => app.publish_draft().await?,
                    KeyCode::Char('e') => app.edit_item()?,
                    KeyCode::Char('d') => app.delete_item().await?,
                    KeyCode::Char('b') => app.backdate_draft().await?,
                    KeyCode::Char('n') => app.new_draft()?,
                    KeyCode::Char('r') => app.refresh().await?,
                    KeyCode::Esc => app.clear_error(),
                    _ => {}
                }
            }
        }
    }
}
