// ABOUTME: Draft push functionality for server-side drafts
// ABOUTME: Handles pushing local drafts to server with post-status: draft

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::client::{MicropubAction, MicropubClient, MicropubRequest};
use crate::config::{load_token, Config};
use crate::draft::Draft;
use crate::media::{find_media_references, replace_paths, resolve_path, upload_file};

#[derive(Debug, Clone, PartialEq)]
pub struct PushResult {
    pub url: String,
    pub is_update: bool,
    pub uploads: Vec<(String, String)>,
}

/// Validate draft_id to prevent path traversal and null byte injection
pub fn validate_draft_id(draft_id: &str) -> Result<()> {
    // Check for null bytes
    if draft_id.contains('\0') {
        bail!("Draft ID contains null byte");
    }

    // Check for path traversal attempts
    if draft_id.contains("..") || draft_id.contains('/') || draft_id.contains('\\') {
        bail!("Draft ID contains invalid path characters");
    }

    // Ensure only alphanumeric, hyphens, and underscores
    if !draft_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Draft ID must contain only alphanumeric characters, hyphens, and underscores");
    }

    Ok(())
}

/// Push a draft to the server as a server-side draft
/// ABOUTME: Loads draft, validates it, and sends to server with post-status: draft
pub async fn cmd_push_draft(draft_id: &str, backdate: Option<DateTime<Utc>>) -> Result<PushResult> {
    // Validate draft_id before using it
    validate_draft_id(draft_id)?;

    // Load draft
    let mut draft = Draft::load(draft_id)?;

    // Load config
    let config = Config::load()?;

    // Determine profile
    let profile_name = draft
        .metadata
        .profile
        .as_deref()
        .or(Some(config.default_profile.as_str()))
        .context("No profile specified and no default profile set")?;

    let profile = config
        .get_profile(profile_name)
        .context(format!("Profile not found: {}", profile_name))?;

    // Load token
    let token = load_token(profile_name)?;

    // Collect media references and deduplicate them
    let mut media_refs_set: HashSet<String> = HashSet::new();

    // Add references from content
    for ref_path in find_media_references(&draft.content) {
        media_refs_set.insert(ref_path);
    }

    // Add local photo references (skip remote URLs)
    for photo_path in &draft.metadata.photo {
        if !photo_path.starts_with("http://") && !photo_path.starts_with("https://") {
            media_refs_set.insert(photo_path.clone());
        }
    }

    // Convert to Vec for iteration
    let media_refs: Vec<String> = media_refs_set.into_iter().collect();

    // Upload media
    let mut replacements = Vec::new();
    let mut uploaded_photo_urls = Vec::new();
    let mut upload_results = Vec::new();

    if !media_refs.is_empty() {
        let media_endpoint = profile.media_endpoint.as_ref().context(format!(
            "No media endpoint found for profile '{}'. Re-authenticate:\n  micropub auth {}",
            profile_name, profile.domain
        ))?;

        println!("Uploading {} media file(s)...", media_refs.len());

        for local_path in media_refs {
            let resolved = resolve_path(&local_path, None)?;
            println!("  Uploading: {}", resolved.display());

            let url = upload_file(media_endpoint, &token, &resolved).await?;
            println!("    -> {}", url);

            let filename = resolved
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            upload_results.push((filename, url.clone()));
            replacements.push((local_path.clone(), url.clone()));

            if draft.metadata.photo.contains(&local_path) {
                uploaded_photo_urls.push(url);
            }
        }
    }

    // Replace paths in content
    let final_content = replace_paths(&draft.content, &replacements);

    // Build micropub request properties
    let mut properties = Map::new();
    properties.insert(
        "content".to_string(),
        Value::Array(vec![Value::String(final_content)]),
    );

    if let Some(name) = &draft.metadata.name {
        properties.insert(
            "name".to_string(),
            Value::Array(vec![Value::String(name.clone())]),
        );
    }

    if !draft.metadata.category.is_empty() {
        properties.insert(
            "category".to_string(),
            Value::Array(
                draft
                    .metadata
                    .category
                    .iter()
                    .map(|c| Value::String(c.clone()))
                    .collect(),
            ),
        );
    }

    if !draft.metadata.photo.is_empty() {
        // Build photo array: uploaded URLs + remote URLs
        let mut photo_values: Vec<Value> = Vec::new();

        for photo_path in &draft.metadata.photo {
            if photo_path.starts_with("http://") || photo_path.starts_with("https://") {
                // Keep remote URLs as-is
                photo_values.push(Value::String(photo_path.clone()));
            } else {
                // Find the corresponding uploaded URL
                if let Some((_, url)) = replacements.iter().find(|(local, _)| local == photo_path) {
                    photo_values.push(Value::String(url.clone()));
                } else {
                    bail!("Photo file not found or not uploaded: {}", photo_path);
                }
            }
        }

        properties.insert("photo".to_string(), Value::Array(photo_values));
    }

    if !draft.metadata.syndicate_to.is_empty() {
        properties.insert(
            "mp-syndicate-to".to_string(),
            Value::Array(
                draft
                    .metadata
                    .syndicate_to
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }

    // Handle published date
    let published_date = backdate.or(draft.metadata.published);
    if let Some(date) = published_date {
        properties.insert(
            "published".to_string(),
            Value::Array(vec![Value::String(date.to_rfc3339())]),
        );
    }

    // CRITICAL: Set post-status to draft
    properties.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("draft".to_string())]),
    );

    // Determine if this is an update or create
    let is_update = draft.metadata.url.is_some();

    // Validate that we're not accidentally overwriting a published post
    if is_update {
        match draft.metadata.status.as_deref() {
            Some("server-draft") | Some("draft") | None => {
                // Safe to update: server-draft, draft, or no status
            }
            Some(status) => {
                bail!(
                    "Cannot push draft with status '{}' - only server-draft or draft status can be updated. \
                     This appears to be a published post.",
                    status
                );
            }
        }
    }

    let request = if is_update {
        // Update existing server draft
        let mut replace = Map::new();
        replace.insert(
            "content".to_string(),
            properties
                .get("content")
                .context("Content property missing when building update request")?
                .clone(),
        );
        if let Some(name) = properties.get("name") {
            replace.insert("name".to_string(), name.clone());
        }
        if let Some(category) = properties.get("category") {
            replace.insert("category".to_string(), category.clone());
        }
        if let Some(photo) = properties.get("photo") {
            replace.insert("photo".to_string(), photo.clone());
        }
        if let Some(published) = properties.get("published") {
            replace.insert("published".to_string(), published.clone());
        }
        if let Some(post_status) = properties.get("post-status") {
            replace.insert("post-status".to_string(), post_status.clone());
        }
        if let Some(syndicate_to) = properties.get("mp-syndicate-to") {
            replace.insert("mp-syndicate-to".to_string(), syndicate_to.clone());
        }

        MicropubRequest {
            action: MicropubAction::Update {
                replace,
                add: Map::new(),
                delete: Vec::new(),
            },
            properties: Map::new(),
            url: draft.metadata.url.clone(),
        }
    } else {
        // Create new server draft
        MicropubRequest {
            action: MicropubAction::Create,
            properties,
            url: None,
        }
    };

    // Send request
    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured for this profile")?;

    let client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Pushing draft to {}...", profile.domain);
    let response = client.send(&request).await?;

    // Update draft metadata
    let server_url = response.url.clone().context("Server didn't return URL")?;
    draft.metadata.status = Some("server-draft".to_string());
    draft.metadata.url = Some(server_url.clone());

    // Save updated draft (stays in drafts directory)
    draft.save().context(
        "Failed to save draft metadata after successful push. \
         The draft was successfully pushed to the server, but local metadata could not be updated.",
    )?;

    println!("✓ Draft pushed successfully!");
    println!("  URL: {}", server_url);

    Ok(PushResult {
        url: server_url,
        is_update,
        uploads: upload_results,
    })
}
