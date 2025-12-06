// ABOUTME: Draft push functionality for server-side drafts
// ABOUTME: Handles pushing local drafts to server with post-status: draft

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

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

/// Push a draft to the server as a server-side draft
/// ABOUTME: Loads draft, validates it, and sends to server with post-status: draft
pub async fn cmd_push_draft(draft_id: &str, backdate: Option<DateTime<Utc>>) -> Result<PushResult> {
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

    // Collect media references
    let mut media_refs = find_media_references(&draft.content);
    for photo_path in &draft.metadata.photo {
        if !photo_path.starts_with("http://") && !photo_path.starts_with("https://") {
            media_refs.push(photo_path.clone());
        }
    }

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
        let photo_values: Vec<Value> = if !uploaded_photo_urls.is_empty() {
            uploaded_photo_urls
                .iter()
                .map(|url| Value::String(url.clone()))
                .collect()
        } else {
            draft
                .metadata
                .photo
                .iter()
                .map(|p| Value::String(p.clone()))
                .collect()
        };
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

    let request = if is_update {
        // Update existing server draft
        let mut replace = Map::new();
        replace.insert(
            "content".to_string(),
            properties.get("content").unwrap().clone(),
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
    draft.save()?;

    println!("✓ Draft pushed successfully!");
    println!("  URL: {}", server_url);

    Ok(PushResult {
        url: server_url,
        is_update,
        uploads: upload_results,
    })
}
