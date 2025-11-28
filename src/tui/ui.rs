// ABOUTME: UI rendering for TUI using ratatui
// ABOUTME: Draws the interface with tabs, lists, and preview panes

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status/help bar
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["[1] Drafts", "[2] Posts", "[3] Media"];
    let selected = match app.current_tab {
        Tab::Drafts => 0,
        Tab::Posts => 1,
        Tab::Media => 2,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Micropub Manager"),
        )
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    match app.current_tab {
        Tab::Drafts => {
            draw_drafts_list(f, app, chunks[0]);
            draw_preview(f, app, chunks[1]);
        }
        Tab::Posts => {
            draw_posts_list(f, app, chunks[0]);
            draw_preview(f, app, chunks[1]);
        }
        Tab::Media => {
            draw_media_list(f, app, chunks[0]);
            draw_preview(f, app, chunks[1]);
        }
    }
}

fn draw_drafts_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .drafts
        .iter()
        .enumerate()
        .map(|(i, draft)| {
            let categories = if draft.categories.is_empty() {
                String::new()
            } else {
                format!(" [{}]", draft.categories.join(", "))
            };

            let content = vec![Line::from(vec![
                Span::raw(&draft.title),
                Span::styled(
                    format!(" ({})", draft.post_type),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(categories, Style::default().fg(Color::Blue)),
            ])];

            let style = if i == app.selected_draft {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Drafts ({})", app.drafts.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(list, area);
}

fn draw_posts_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .posts
        .iter()
        .enumerate()
        .map(|(i, post)| {
            // Show published date and name/preview
            let date_part = if post.published.len() >= 10 {
                &post.published[..10] // YYYY-MM-DD
            } else {
                &post.published
            };

            let content_part = if let Some(ref name) = post.name {
                name.clone()
            } else {
                // No name - try to extract from URL, fallback to [no title]
                post.url
                    .split('/')
                    .next_back()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::from("[no title]"))
            };

            let mut display = format!("{} - {}", date_part, content_part);

            if !post.categories.is_empty() {
                display.push_str(&format!(" [{}]", post.categories.join(", ")));
            }

            let style = if i == app.selected_post {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(display).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Posts ({})", app.posts.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(list, area);
}

fn draw_media_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .media
        .iter()
        .enumerate()
        .map(|(i, media)| {
            // Show upload date and filename/name
            let date_part = if media.uploaded.len() >= 10 {
                &media.uploaded[..10] // YYYY-MM-DD
            } else {
                &media.uploaded
            };

            let display_name = if let Some(ref name) = media.name {
                if name.is_empty() {
                    // Try to extract filename from URL
                    media
                        .url
                        .split('/')
                        .next_back()
                        .unwrap_or("[unknown]")
                        .to_string()
                } else {
                    name.clone()
                }
            } else {
                // Try to extract filename from URL
                media
                    .url
                    .split('/')
                    .next_back()
                    .unwrap_or("[unknown]")
                    .to_string()
            };

            let display = format!("{} - {}", date_part, display_name);

            let style = if i == app.selected_media {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(display).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Media ({})", app.media.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(list, area);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(ref preview) = app.preview_content {
        preview.clone()
    } else if app.drafts.is_empty() {
        "No drafts found.\n\nCreate a new draft with: micropub draft new".to_string()
    } else {
        "No preview available".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.awaiting_date_input() {
        "[Enter] Submit  [Esc] Cancel  [Backspace] Delete"
    } else if app.awaiting_confirmation() {
        "[y] Yes  [n] No"
    } else {
        match app.current_tab {
            Tab::Drafts => "[p]ublish [b]ackdate [e]dit [d]elete [n]ew [r]efresh [q]uit",
            Tab::Posts => "[r]efresh [q]uit",
            Tab::Media => "[r]efresh [q]uit",
        }
    };

    let text = if let Some(ref error) = app.error_message {
        vec![
            Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(error),
            ]),
            Line::from(Span::styled(
                "[Esc] to dismiss",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else if app.awaiting_date_input() {
        let prompt = app.status_message.as_deref().unwrap_or("");
        vec![
            Line::from(vec![Span::styled(
                prompt,
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Green)),
                Span::styled(
                    &app.date_input,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("_", Style::default().fg(Color::White)),
            ]),
        ]
    } else if let Some(ref status) = app.status_message {
        vec![Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Green)),
            Span::raw(status),
        ])]
    } else {
        vec![Line::from(Span::styled(
            help_text,
            Style::default().fg(Color::Gray),
        ))]
    };

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}
