// ABOUTME: Post management operations (update, delete, undelete, whoami, list)
// ABOUTME: Handles modifications to existing posts and queries

use anyhow::{Context, Result};
use is_terminal::IsTerminal;
use reqwest::Client as HttpClient;
use serde_json::{Map, Value};
use std::io::{self, Write};

use crate::client::{MicropubAction, MicropubClient, MicropubRequest};
use crate::config::{load_token, Config};

/// Helper function to prompt user for showing more results
fn prompt_for_more() -> Result<bool> {
    if !io::stdout().is_terminal() {
        return Ok(false);
    }

    print!("Show more results? [y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

pub async fn cmd_update(post_url: &str) -> Result<()> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No default profile set. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    // First, fetch the current post content
    println!("Fetching post: {}", post_url);
    let client = HttpClient::new();
    let response = client
        .get(format!("{}?q=source&url={}", micropub_endpoint, post_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to fetch post")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unable to read response>"));
        anyhow::bail!("Failed to fetch post: HTTP {}\n{}", status, body);
    }

    let source: Value = response.json().await.context("Failed to parse post data")?;

    // Extract properties
    let properties = source
        .get("properties")
        .and_then(|v| v.as_object())
        .context("Post has no properties")?;

    // Convert to editable format (similar to draft)
    let content = properties
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let name = properties
        .get("name")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str());

    let categories: Vec<String> = properties
        .get("category")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Create a temporary file for editing
    let mut editable_content = String::new();
    editable_content.push_str("---\n");
    if let Some(title) = name {
        editable_content.push_str(&format!("title: \"{}\"\n", title));
    }
    if !categories.is_empty() {
        editable_content.push_str("category:\n");
        for cat in &categories {
            editable_content.push_str(&format!("  - {}\n", cat));
        }
    }
    editable_content.push_str("---\n");
    editable_content.push_str(content);

    // Write to temp file and open editor
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("micropub-update-{}.md", uuid::Uuid::new_v4()));
    std::fs::write(&temp_file, &editable_content)?;

    // Open editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&temp_file)
        .status()
        .context("Failed to open editor")?;

    if !status.success() {
        anyhow::bail!("Editor exited with error");
    }

    // Read back the edited content
    let edited_content = std::fs::read_to_string(&temp_file)?;
    std::fs::remove_file(&temp_file)?;

    // Parse the edited content
    let (edited_frontmatter, edited_body) = if edited_content.starts_with("---\n") {
        let parts: Vec<&str> = edited_content.splitn(3, "---\n").collect();
        if parts.len() >= 3 {
            (parts[1], parts[2])
        } else {
            ("", edited_content.as_str())
        }
    } else {
        ("", edited_content.as_str())
    };

    // Parse frontmatter for changes
    let mut replace = Map::new();

    // Always replace content if it changed
    if edited_body.trim() != content.trim() {
        replace.insert(
            "content".to_string(),
            Value::Array(vec![Value::String(edited_body.trim().to_string())]),
        );
    }

    // Parse title
    if let Some(title_line) = edited_frontmatter.lines().find(|l| l.starts_with("title:")) {
        let title = title_line
            .trim_start_matches("title:")
            .trim()
            .trim_matches('"');
        if Some(title) != name {
            replace.insert(
                "name".to_string(),
                Value::Array(vec![Value::String(title.to_string())]),
            );
        }
    } else if name.is_some() {
        // Title was removed
        replace.insert("name".to_string(), Value::Array(vec![]));
    }

    // Parse categories
    let mut new_categories = Vec::new();
    let mut in_category = false;
    for line in edited_frontmatter.lines() {
        if line.starts_with("category:") {
            in_category = true;
        } else if in_category && line.trim().starts_with("- ") {
            new_categories.push(line.trim_start_matches("- ").trim().to_string());
        } else if in_category && !line.trim().is_empty() && !line.starts_with(" ") {
            in_category = false;
        }
    }

    if new_categories != categories {
        replace.insert(
            "category".to_string(),
            Value::Array(
                new_categories
                    .iter()
                    .map(|c| Value::String(c.clone()))
                    .collect(),
            ),
        );
    }

    if replace.is_empty() {
        println!("No changes detected.");
        return Ok(());
    }

    // Send update request
    let request = MicropubRequest {
        action: MicropubAction::Update {
            replace,
            add: Map::new(),
            delete: Vec::new(),
        },
        properties: Map::new(),
        url: Some(post_url.to_string()),
    };

    let micropub_client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Updating post...");
    micropub_client.send(&request).await?;

    println!("✓ Post updated successfully!");

    Ok(())
}

pub async fn cmd_delete(post_url: &str) -> Result<()> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No default profile set. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    let request = MicropubRequest {
        action: MicropubAction::Delete,
        properties: Map::new(),
        url: Some(post_url.to_string()),
    };

    let client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Deleting post: {}", post_url);
    client.send(&request).await?;

    println!("✓ Post deleted successfully");

    Ok(())
}

pub async fn cmd_undelete(post_url: &str) -> Result<()> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No default profile set. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    let request = MicropubRequest {
        action: MicropubAction::Undelete,
        properties: Map::new(),
        url: Some(post_url.to_string()),
    };

    let client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Undeleting post: {}", post_url);
    client.send(&request).await?;

    println!("✓ Post undeleted successfully");

    Ok(())
}

pub async fn cmd_whoami() -> Result<()> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No profile configured. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    // Query the micropub endpoint for user info
    let client = HttpClient::new();
    let response = client
        .get(format!("{}?q=config", micropub_endpoint))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to query micropub endpoint")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get user info: HTTP {}", response.status());
    }

    println!("Authenticated as:");
    println!("  Profile: {}", profile_name);
    println!("  Domain: {}", profile.domain);
    println!("  Micropub endpoint: {}", micropub_endpoint);

    if let Some(media) = &profile.media_endpoint {
        println!("  Media endpoint: {}", media);
    }

    Ok(())
}

/// Fetch posts from the micropub endpoint and return them as structured data
pub async fn fetch_posts(limit: usize, offset: usize) -> Result<Vec<PostData>> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No profile configured. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    let client = HttpClient::new();
    let mut url = format!("{}?q=source&limit={}", micropub_endpoint, limit);
    if offset > 0 {
        url.push_str(&format!("&offset={}", offset));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to query posts")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unable to read response>"));
        anyhow::bail!("Failed to list posts: HTTP {}\n{}", status, body);
    }

    let data: Value = response.json().await.context("Failed to parse response")?;

    let mut posts = Vec::new();

    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let properties = item
                .get("properties")
                .context("Missing properties in post")?;

            let url = properties
                .get("url")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no URL)")
                .to_string();

            let content = properties
                .get("content")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let name = properties
                .get("name")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(String::from);

            let published = properties
                .get("published")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no date)")
                .to_string();

            let categories: Vec<String> = properties
                .get("category")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            posts.push(PostData {
                url,
                content,
                name,
                published,
                categories,
            });
        }
    }

    Ok(posts)
}

pub struct PostData {
    pub url: String,
    pub content: String,
    pub name: Option<String>,
    pub published: String,
    pub categories: Vec<String>,
}

pub async fn cmd_list_posts(limit: usize, offset: usize) -> Result<()> {
    let mut current_offset = offset;
    let mut first_page = true;

    loop {
        let posts = fetch_posts(limit, current_offset).await?;

        if posts.is_empty() {
            if first_page {
                println!("No posts found.");
            } else {
                println!("No more posts.");
            }
            return Ok(());
        }

        if first_page {
            println!("Recent posts:");
            println!();
        }

        for (idx, post) in posts.iter().enumerate() {
            let display_content = post.name.as_ref().unwrap_or(&post.content);
            let content_preview = if display_content.len() > 80 {
                format!("{}...", &display_content[..77])
            } else {
                display_content.to_string()
            };

            println!("{}. {}", current_offset + idx + 1, content_preview);
            println!("   URL: {}", post.url);
            println!("   Published: {}", post.published);
            println!();
        }

        let has_more = posts.len() == limit;
        if !has_more {
            return Ok(());
        }

        if !prompt_for_more()? {
            return Ok(());
        }

        current_offset += limit;
        first_page = false;
    }
}

/// Fetch media from the micropub endpoint and return them as structured data
pub async fn fetch_media(limit: usize, offset: usize) -> Result<Vec<MediaData>> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No profile configured. Run 'micropub auth' first");
    }

    let profile = config
        .get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured")?;

    let client = HttpClient::new();
    let mut url = format!(
        "{}?q=source&limit={}&filter=photo",
        micropub_endpoint, limit
    );
    if offset > 0 {
        url.push_str(&format!("&offset={}", offset));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to query media")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unable to read response>"));
        anyhow::bail!("Failed to list media: HTTP {}\n{}", status, body);
    }

    let data: Value = response.json().await.context("Failed to parse response")?;

    let mut media_items = Vec::new();

    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let properties = item
                .get("properties")
                .context("Missing properties in media item")?;

            // Get URL
            let url = properties
                .get("url")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .or_else(|| {
                    properties
                        .get("photo")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("(no URL)")
                .to_string();

            // Get published date
            let uploaded = properties
                .get("published")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no date)")
                .to_string();

            // Get name/alt text if available
            let name = properties
                .get("name")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(String::from);

            media_items.push(MediaData {
                url,
                name,
                uploaded,
            });
        }
    }

    Ok(media_items)
}

pub struct MediaData {
    pub url: String,
    pub name: Option<String>,
    pub uploaded: String,
}

pub async fn cmd_list_media(limit: usize, offset: usize) -> Result<()> {
    let mut current_offset = offset;
    let mut first_page = true;

    loop {
        let media_items = fetch_media(limit, current_offset).await?;

        if media_items.is_empty() {
            if first_page {
                println!("No media files found.");
            } else {
                println!("No more media files.");
            }
            return Ok(());
        }

        if first_page {
            println!("Recent media uploads:");
            println!();
        }

        for (idx, item) in media_items.iter().enumerate() {
            println!("{}. {}", current_offset + idx + 1, item.url);
            if let Some(ref n) = item.name {
                println!("   Name: {}", n);
            }
            println!("   Uploaded: {}", item.uploaded);
            println!();
        }

        // Check if there might be more results and if we should prompt
        let has_more = media_items.len() == limit;
        if !has_more {
            return Ok(());
        }

        // Prompt for more results if in TTY mode
        if !prompt_for_more()? {
            return Ok(());
        }

        current_offset += limit;
        first_page = false;
    }
}
