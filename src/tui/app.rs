// ABOUTME: Application state and event handling for TUI
// ABOUTME: Manages tabs, items, selections, and user actions

use anyhow::Result;

use crate::config::get_drafts_dir;
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
    pub name: Option<String>,
    pub published: String,
    pub categories: Vec<String>,
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
    BackdateDraft(String),
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
    pub date_input: String,
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
            date_input: String::new(),
        };

        app.load_drafts()?;
        app.load_posts().await?;
        app.load_media().await?;
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

    async fn load_posts(&mut self) -> Result<()> {
        self.posts.clear();

        match crate::operations::fetch_posts(20, 0).await {
            Ok(posts) => {
                for post in posts {
                    self.posts.push(PostItem {
                        url: post.url,
                        content: post.content,
                        name: post.name,
                        published: post.published,
                        categories: post.categories,
                    });
                }
                Ok(())
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load posts: {}", e));
                Ok(())
            }
        }
    }

    async fn load_media(&mut self) -> Result<()> {
        self.media.clear();

        match crate::operations::fetch_media(20, 0).await {
            Ok(media_items) => {
                for media in media_items {
                    self.media.push(MediaItem {
                        url: media.url,
                        name: media.name,
                        uploaded: media.uploaded,
                    });
                }
                Ok(())
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load media: {}", e));
                Ok(())
            }
        }
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
                self.load_posts().await?;
                self.update_preview();
                self.status_message = Some("Posts refreshed".to_string());
            }
            Tab::Media => {
                self.load_media().await?;
                self.update_preview();
                self.status_message = Some("Media refreshed".to_string());
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
                    self.update_preview();
                }
            }
            Tab::Media => {
                if !self.media.is_empty() {
                    self.selected_media = (self.selected_media + 1) % self.media.len();
                    self.update_preview();
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
                    self.update_preview();
                }
            }
            Tab::Media => {
                if !self.media.is_empty() {
                    self.selected_media = if self.selected_media == 0 {
                        self.media.len() - 1
                    } else {
                        self.selected_media - 1
                    };
                    self.update_preview();
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
        } else if self.current_tab == Tab::Posts && !self.posts.is_empty() {
            if let Some(post_item) = self.posts.get(self.selected_post) {
                let mut preview = String::new();

                if let Some(ref name) = post_item.name {
                    preview.push_str(&format!("Title: {}\n\n", name));
                }

                preview.push_str(&format!("URL: {}\n", post_item.url));
                preview.push_str(&format!("Published: {}\n", post_item.published));

                if !post_item.categories.is_empty() {
                    preview.push_str(&format!(
                        "Categories: {}\n",
                        post_item.categories.join(", ")
                    ));
                }

                preview.push_str("\n---\n\n");
                preview.push_str(&post_item.content);

                self.preview_content = Some(preview);
            }
        } else if self.current_tab == Tab::Media && !self.media.is_empty() {
            if let Some(media_item) = self.media.get(self.selected_media) {
                let mut preview = String::new();

                preview.push_str(&format!("URL: {}\n", media_item.url));
                preview.push_str(&format!("Uploaded: {}\n", media_item.uploaded));

                if let Some(ref name) = media_item.name {
                    preview.push_str(&format!("\nName/Alt Text:\n{}\n", name));
                }

                self.preview_content = Some(preview);
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

    pub fn edit_item(&mut self) -> Result<Option<String>> {
        match self.current_tab {
            Tab::Drafts => {
                if let Some(draft_item) = self.drafts.get(self.selected_draft) {
                    // Return draft ID for TUI to handle suspend/resume
                    Ok(Some(draft_item.id.clone()))
                } else {
                    Ok(None)
                }
            }
            _ => {
                self.error_message = Some("Edit not available for this view".to_string());
                Ok(None)
            }
        }
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
        if self.current_tab != Tab::Drafts || self.drafts.is_empty() {
            return Ok(());
        }

        if let Some(draft_item) = self.drafts.get(self.selected_draft) {
            self.confirmation_action = ConfirmationAction::BackdateDraft(draft_item.id.clone());
            self.date_input.clear();
            self.status_message =
                Some("Enter date (ISO 8601, e.g., 2024-01-15T10:30:00Z):".to_string());
        }

        Ok(())
    }

    pub fn new_draft(&mut self) -> Result<String> {
        // Generate new draft ID and return it for TUI to handle
        Ok(crate::draft::generate_draft_id())
    }

    pub fn reload_and_select_draft(&mut self, draft_id: &str) -> Result<()> {
        // Reload drafts
        self.load_drafts()?;

        // Find and select the new draft
        if let Some(index) = self.drafts.iter().position(|d| d.id == draft_id) {
            self.selected_draft = index;
            self.update_preview();
            self.status_message = Some(format!("Draft created: {}", draft_id));
        } else {
            self.error_message = Some("Draft created but not found in list".to_string());
        }

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
            ConfirmationAction::BackdateDraft(draft_id) => {
                // Parse the date from date_input
                use chrono::DateTime;
                match DateTime::parse_from_rfc3339(&self.date_input) {
                    Ok(parsed_date) => {
                        self.status_message = Some("Publishing with backdate...".to_string());
                        let parsed_date_utc = parsed_date.with_timezone(&chrono::Utc);

                        let draft_path = get_drafts_dir()?.join(format!("{}.md", draft_id));
                        let draft_path_str = draft_path.to_string_lossy().to_string();

                        match crate::publish::cmd_publish(&draft_path_str, Some(parsed_date_utc))
                            .await
                        {
                            Ok(_) => {
                                self.status_message =
                                    Some("Draft published with backdate successfully!".to_string());
                                self.load_drafts()?;
                                if self.selected_draft >= self.drafts.len()
                                    && self.selected_draft > 0
                                {
                                    self.selected_draft -= 1;
                                }
                                self.update_preview();
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to publish: {}", e));
                            }
                        }
                    }
                    Err(_) => {
                        self.error_message = Some(
                            "Invalid date format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)"
                                .to_string(),
                        );
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
        self.date_input.clear();
        Ok(())
    }

    pub fn cancel_action(&mut self) {
        self.confirmation_action = ConfirmationAction::None;
        self.date_input.clear();
        self.status_message = Some("Action cancelled".to_string());
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.status_message = None;
        self.quit_requested = false;
    }

    pub fn awaiting_date_input(&self) -> bool {
        matches!(
            self.confirmation_action,
            ConfirmationAction::BackdateDraft(_)
        )
    }

    pub fn add_date_char(&mut self, c: char) {
        self.date_input.push(c);
    }

    pub fn delete_date_char(&mut self) {
        self.date_input.pop();
    }
}
