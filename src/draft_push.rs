// ABOUTME: Draft push functionality for server-side drafts
// ABOUTME: Handles pushing local drafts to server with post-status: draft

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::draft::Draft;

#[derive(Debug, Clone, PartialEq)]
pub struct PushResult {
    pub url: String,
    pub is_update: bool,
    pub uploads: Vec<(String, String)>,
}

/// Push a draft to the server as a server-side draft
/// ABOUTME: Loads draft, validates it, and sends to server with post-status: draft
pub async fn cmd_push_draft(
    draft_id: &str,
    _backdate: Option<DateTime<Utc>>,
) -> Result<PushResult> {
    // Load draft
    let _draft = Draft::load(draft_id)?;

    // TODO: Implement push logic
    anyhow::bail!("Push not yet implemented")
}
