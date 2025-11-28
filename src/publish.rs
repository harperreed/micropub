// ABOUTME: Post publishing functionality
// ABOUTME: Orchestrates draft loading, media upload, and micropub posting

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::client::{MicropubAction, MicropubClient, MicropubRequest};
use crate::config::{load_token, Config};
use crate::draft::Draft;
use crate::media::{find_media_references, replace_paths, resolve_path, upload_file};

pub async fn cmd_publish(draft_path: &str, backdate: Option<DateTime<Utc>>) -> Result<()> {
    // Extract draft ID from path
    let draft_id = std::path::Path::new(draft_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid draft path")?;

    // Load draft
    let mut draft = Draft::load(draft_id)?;

    // Load config
    let config = Config::load()?;

    // Determine which profile to use
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

    // Collect all media files to upload (from content and photo metadata)
    let mut media_refs = find_media_references(&draft.content);

    // Also include photos from metadata that look like local paths
    for photo_path in &draft.metadata.photo {
        // Check if it's a local path (not a URL)
        if !photo_path.starts_with("http://") && !photo_path.starts_with("https://") {
            media_refs.push(photo_path.clone());
        }
    }

    let mut replacements = Vec::new();
    let mut uploaded_photo_urls = Vec::new();

    if !media_refs.is_empty() {
        let media_endpoint = profile.media_endpoint.as_ref()
            .context(format!(
                "No media endpoint found for profile '{}'. Re-authenticate to discover media endpoint:\n  micropub auth {}",
                profile_name, profile.domain
            ))?;

        println!("Uploading {} media file(s)...", media_refs.len());

        for local_path in media_refs {
            let resolved = resolve_path(&local_path, None)?;
            println!("  Uploading: {}", resolved.display());

            let url = upload_file(media_endpoint, &token, &resolved).await?;
            println!("    -> {}", url);

            replacements.push((local_path.clone(), url.clone()));

            // If this was from photo metadata, save the URL
            if draft.metadata.photo.contains(&local_path) {
                uploaded_photo_urls.push(url);
            }
        }
    }

    // Replace local paths with URLs in content
    let final_content = replace_paths(&draft.content, &replacements);

    // Build micropub request
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

    // Use uploaded photo URLs if we have them, otherwise use original values (for URLs)
    if !draft.metadata.photo.is_empty() {
        let photo_values: Vec<Value> = if !uploaded_photo_urls.is_empty() {
            // Use uploaded URLs
            uploaded_photo_urls
                .iter()
                .map(|url| Value::String(url.clone()))
                .collect()
        } else {
            // Keep original values (they must already be URLs)
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

    // Handle published date (backdate or from draft)
    let published_date = backdate.or(draft.metadata.published);
    if let Some(date) = published_date {
        properties.insert(
            "published".to_string(),
            Value::Array(vec![Value::String(date.to_rfc3339())]),
        );
    }

    let request = MicropubRequest {
        action: MicropubAction::Create,
        properties,
        url: None,
    };

    // Send request
    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured for this profile")?;

    let client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Publishing to {}...", profile.domain);
    let response = client.send(&request).await?;

    // Archive draft with metadata
    draft.metadata.status = Some("published".to_string());
    draft.metadata.url = response.url.clone();
    draft.metadata.published_at = Some(Utc::now());

    let archive_path = draft.archive()?;

    println!("✓ Published successfully!");
    if let Some(url) = response.url {
        println!("  URL: {}", url);
    }
    println!("  Draft archived to: {}", archive_path.display());

    Ok(())
}
