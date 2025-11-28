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
async fn run_app<B: Backend + io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // Handle date input mode
                if app.awaiting_date_input() {
                    match key.code {
                        KeyCode::Enter => {
                            app.confirm_action().await?;
                        }
                        KeyCode::Esc => {
                            app.cancel_action();
                        }
                        KeyCode::Backspace => {
                            app.delete_date_char();
                        }
                        KeyCode::Char(c) => {
                            app.add_date_char(c);
                        }
                        _ => {}
                    }
                } else {
                    // Normal key handling
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
                        KeyCode::Char('e') => {
                            // Suspend TUI to edit draft
                            match app.edit_item() {
                                Ok(Some(draft_id)) => {
                                    if let Err(e) =
                                        suspend_and_edit_draft(terminal, &draft_id).await
                                    {
                                        app.error_message =
                                            Some(format!("Failed to edit draft: {}", e));
                                    } else {
                                        // Reload drafts and select the edited one
                                        if let Err(e) = app.reload_and_select_draft(&draft_id) {
                                            app.error_message =
                                                Some(format!("Failed to reload drafts: {}", e));
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // No draft to edit, error already set in edit_item
                                }
                                Err(e) => {
                                    app.error_message =
                                        Some(format!("Failed to get draft for editing: {}", e));
                                }
                            }
                        }
                        KeyCode::Char('d') => app.delete_item().await?,
                        KeyCode::Char('b') => app.backdate_draft().await?,
                        KeyCode::Char('n') => {
                            // Suspend TUI to create new draft
                            match app.new_draft() {
                                Ok(draft_id) => {
                                    if let Err(e) =
                                        suspend_and_create_draft(terminal, &draft_id).await
                                    {
                                        app.error_message =
                                            Some(format!("Failed to create draft: {}", e));
                                    } else {
                                        // Reload drafts and select the new one
                                        if let Err(e) = app.reload_and_select_draft(&draft_id) {
                                            app.error_message =
                                                Some(format!("Failed to reload drafts: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    app.error_message =
                                        Some(format!("Failed to generate draft ID: {}", e));
                                }
                            }
                        }
                        KeyCode::Char('r') => app.refresh().await?,
                        KeyCode::Esc => app.clear_error(),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Suspend the TUI, edit an existing draft, then resume TUI
async fn suspend_and_edit_draft<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    draft_id: &str,
) -> Result<()> {
    use crate::config::get_drafts_dir;
    use crate::config::Config;
    use std::process::Command;

    // Validate draft ID to prevent path traversal
    if draft_id.contains('/') || draft_id.contains('\\') || draft_id.contains("..") {
        anyhow::bail!("Invalid draft ID: {}", draft_id);
    }

    let path = get_drafts_dir()?.join(format!("{}.md", draft_id));

    if !path.exists() {
        anyhow::bail!("Draft not found: {}", draft_id);
    }

    // Suspend TUI
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    // Open in editor
    let config = Config::load()?;
    let editor = config
        .editor
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vim".to_string());

    let status = Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        // Resume TUI even on error
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        anyhow::bail!("Editor exited with error");
    }

    // Resume TUI
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    Ok(())
}

/// Suspend the TUI, create a draft and open editor, then resume TUI
async fn suspend_and_create_draft<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    draft_id: &str,
) -> Result<()> {
    use crate::config::Config;
    use crate::draft::Draft;
    use std::process::Command;

    // Suspend TUI
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    // Create and save initial draft
    let draft = Draft::new(draft_id.to_string());
    let path = draft.save()?;

    // Open in editor
    let config = Config::load()?;
    let editor = config
        .editor
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vim".to_string());

    let status = Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        // Resume TUI even on error
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        anyhow::bail!("Editor exited with error");
    }

    // Resume TUI
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    Ok(())
}
