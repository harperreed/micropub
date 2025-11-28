// ABOUTME: Application state and event handling for TUI
// ABOUTME: Manages tabs, items, selections, and user actions

use anyhow::Result;

use crate::config::{get_drafts_dir, Config};
use crate::draft::Draft;

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Drafts,
    Posts,
    Media,
}

#[derive(Debug, Clone)]
pub struct DraftItem {
    pub id: String,
    pub title: String,
    pub post_type: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PostItem {
    pub url: String,
    pub content: String,
    pub published: String,
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub url: String,
    pub name: Option<String>,
    pub uploaded: String,
}

pub enum ConfirmationAction {
    DeleteDraft(String),
    PublishDraft(String),
    None,
}

pub struct App {
    pub current_tab: Tab,
    pub drafts: Vec<DraftItem>,
    pub posts: Vec<PostItem>,
    pub media: Vec<MediaItem>,
    pub selected_draft: usize,
    pub selected_post: usize,
    pub selected_media: usize,
    pub preview_content: Option<String>,
    pub error_message: Option<String>,
    pub status_message: Option<String>,
    pub confirmation_action: ConfirmationAction,
    pub quit_requested: bool,
}

impl App {
    pub async fn new() -> Result<Self> {
        let mut app = App {
            current_tab: Tab::Drafts,
            drafts: Vec::new(),
            posts: Vec::new(),
            media: Vec::new(),
            selected_draft: 0,
            selected_post: 0,
            selected_media: 0,
            preview_content: None,
            error_message: None,
            status_message: None,
            confirmation_action: ConfirmationAction::None,
            quit_requested: false,
        };

        app.load_drafts()?;
        app.update_preview();
        Ok(app)
    }

    fn load_drafts(&mut self) -> Result<()> {
        self.drafts.clear();
        let draft_ids = Draft::list_all()?;

        for id in draft_ids {
            if let Ok(draft) = Draft::load(&id) {
                let title = draft
                    .metadata
                    .name
                    .unwrap_or_else(|| "[untitled]".to_string());
                self.drafts.push(DraftItem {
                    id: id.clone(),
                    title,
                    post_type: draft.metadata.post_type.clone(),
                    categories: draft.metadata.category.clone(),
                });
            }
        }

        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.status_message = Some("Refreshing...".to_string());
        match self.current_tab {
            Tab::Drafts => {
                self.load_drafts()?;
                self.update_preview();
                self.status_message = Some("Drafts refreshed".to_string());
            }
            Tab::Posts => {
                // Note: We can't easily refresh posts without refactoring cmd_list_posts
                // to return data instead of printing it
                self.status_message = Some("Posts view (refresh not yet implemented)".to_string());
            }
            Tab::Media => {
                self.status_message = Some("Media view (refresh not yet implemented)".to_string());
            }
        }
        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Drafts => Tab::Posts,
            Tab::Posts => Tab::Media,
            Tab::Media => Tab::Drafts,
        };
        self.update_preview();
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Drafts => Tab::Media,
            Tab::Posts => Tab::Drafts,
            Tab::Media => Tab::Posts,
        };
        self.update_preview();
    }

    pub fn next_item(&mut self) {
        match self.current_tab {
            Tab::Drafts => {
                if !self.drafts.is_empty() {
                    self.selected_draft = (self.selected_draft + 1) % self.drafts.len();
                    self.update_preview();
                }
            }
            Tab::Posts => {
                if !self.posts.is_empty() {
                    self.selected_post = (self.selected_post + 1) % self.posts.len();
                }
            }
            Tab::Media => {
                if !self.media.is_empty() {
                    self.selected_media = (self.selected_media + 1) % self.media.len();
                }
            }
        }
    }

    pub fn previous_item(&mut self) {
        match self.current_tab {
            Tab::Drafts => {
                if !self.drafts.is_empty() {
                    self.selected_draft = if self.selected_draft == 0 {
                        self.drafts.len() - 1
                    } else {
                        self.selected_draft - 1
                    };
                    self.update_preview();
                }
            }
            Tab::Posts => {
                if !self.posts.is_empty() {
                    self.selected_post = if self.selected_post == 0 {
                        self.posts.len() - 1
                    } else {
                        self.selected_post - 1
                    };
                }
            }
            Tab::Media => {
                if !self.media.is_empty() {
                    self.selected_media = if self.selected_media == 0 {
                        self.media.len() - 1
                    } else {
                        self.selected_media - 1
                    };
                }
            }
        }
    }

    fn update_preview(&mut self) {
        self.preview_content = None;

        if self.current_tab == Tab::Drafts && !self.drafts.is_empty() {
            if let Some(draft_item) = self.drafts.get(self.selected_draft) {
                if let Ok(draft) = Draft::load(&draft_item.id) {
                    if let Ok(content) = draft.to_string() {
                        self.preview_content = Some(content);
                    }
                }
            }
        }
    }

    pub async fn select_item(&mut self) -> Result<()> {
        // For now, selection just updates preview (already done by navigation)
        self.status_message = Some("Item selected".to_string());
        Ok(())
    }

    pub async fn publish_draft(&mut self) -> Result<()> {
        if self.current_tab != Tab::Drafts || self.drafts.is_empty() {
            return Ok(());
        }

        if let Some(draft_item) = self.drafts.get(self.selected_draft) {
            self.confirmation_action = ConfirmationAction::PublishDraft(draft_item.id.clone());
            self.status_message = Some("Publish draft? (y/n)".to_string());
        }

        Ok(())
    }

    pub fn edit_item(&mut self) -> Result<()> {
        match self.current_tab {
            Tab::Drafts => {
                if let Some(_draft_item) = self.drafts.get(self.selected_draft) {
                    let _config = Config::load()?;
                    let _editor = _config
                        .editor
                        .or_else(|| std::env::var("EDITOR").ok())
                        .unwrap_or_else(|| "vim".to_string());

                    let _path = get_drafts_dir()?.join(format!("{}.md", _draft_item.id));

                    // We need to exit the TUI temporarily to open the editor
                    // This is a limitation - for now we'll just show an error
                    self.error_message = Some(
                        "Edit mode not supported in TUI. Use 'micropub draft edit <id>'"
                            .to_string(),
                    );
                }
            }
            _ => {
                self.error_message = Some("Edit not available for this view".to_string());
            }
        }

        Ok(())
    }

    pub async fn delete_item(&mut self) -> Result<()> {
        if self.current_tab != Tab::Drafts || self.drafts.is_empty() {
            return Ok(());
        }

        if let Some(draft_item) = self.drafts.get(self.selected_draft) {
            self.confirmation_action = ConfirmationAction::DeleteDraft(draft_item.id.clone());
            self.status_message = Some("Delete draft? (y/n)".to_string());
        }

        Ok(())
    }

    pub async fn backdate_draft(&mut self) -> Result<()> {
        self.error_message =
            Some("Backdate not yet implemented in TUI. Use 'micropub backdate'".to_string());
        Ok(())
    }

    pub fn new_draft(&mut self) -> Result<()> {
        self.error_message =
            Some("New draft not yet implemented in TUI. Use 'micropub draft new'".to_string());
        Ok(())
    }

    pub fn confirm_quit(&mut self) -> bool {
        if !self.quit_requested {
            self.quit_requested = true;
            self.status_message = Some("Press 'q' again to quit".to_string());
            false
        } else {
            true
        }
    }

    pub fn awaiting_confirmation(&self) -> bool {
        !matches!(self.confirmation_action, ConfirmationAction::None)
    }

    pub async fn confirm_action(&mut self) -> Result<()> {
        match &self.confirmation_action {
            ConfirmationAction::PublishDraft(draft_id) => {
                self.status_message = Some("Publishing...".to_string());

                // Load the draft and publish it
                let draft_path = get_drafts_dir()?.join(format!("{}.md", draft_id));
                let draft_path_str = draft_path.to_string_lossy().to_string();

                match crate::publish::cmd_publish(&draft_path_str, None).await {
                    Ok(_) => {
                        self.status_message = Some("Draft published successfully!".to_string());
                        self.load_drafts()?;
                        if self.selected_draft >= self.drafts.len() && self.selected_draft > 0 {
                            self.selected_draft -= 1;
                        }
                        self.update_preview();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to publish: {}", e));
                    }
                }
            }
            ConfirmationAction::DeleteDraft(draft_id) => {
                let draft_path = get_drafts_dir()?.join(format!("{}.md", draft_id));
                match std::fs::remove_file(&draft_path) {
                    Ok(_) => {
                        self.status_message = Some("Draft deleted".to_string());
                        self.load_drafts()?;
                        if self.selected_draft >= self.drafts.len() && self.selected_draft > 0 {
                            self.selected_draft -= 1;
                        }
                        self.update_preview();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to delete: {}", e));
                    }
                }
            }
            ConfirmationAction::None => {}
        }

        self.confirmation_action = ConfirmationAction::None;
        Ok(())
    }

    pub fn cancel_action(&mut self) {
        self.confirmation_action = ConfirmationAction::None;
        self.status_message = Some("Action cancelled".to_string());
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.status_message = None;
        self.quit_requested = false;
    }
}
