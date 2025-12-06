// ABOUTME: Draft push functionality for server-side drafts
// ABOUTME: Handles pushing local drafts to server with post-status: draft

#[derive(Debug, Clone, PartialEq)]
pub struct PushResult {
    pub url: String,
    pub is_update: bool,
    pub uploads: Vec<(String, String)>,
}
