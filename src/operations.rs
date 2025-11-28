// ABOUTME: Post management operations (update, delete, undelete, whoami, list)
// ABOUTME: Handles modifications to existing posts and queries

use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{Map, Value};

use crate::client::{MicropubAction, MicropubClient, MicropubRequest};
use crate::config::{load_token, Config};

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

pub async fn cmd_list_posts(limit: usize) -> Result<()> {
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

    // Query for posts using the source query
    let client = HttpClient::new();
    let response = client
        .get(format!("{}?q=source&limit={}", micropub_endpoint, limit))
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

    // The response format can vary, but typically has "items" array
    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        if items.is_empty() {
            println!("No posts found.");
            return Ok(());
        }

        println!("Recent posts:");
        println!();

        for (idx, item) in items.iter().enumerate() {
            let properties = item
                .get("properties")
                .context("Missing properties in post")?;

            // Get URL
            let url = properties
                .get("url")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no URL)");

            // Get content or name
            let content = properties
                .get("content")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .or_else(|| {
                    properties
                        .get("name")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("(no content)");

            // Get published date
            let published = properties
                .get("published")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no date)");

            // Truncate content for display
            let content_preview = if content.len() > 80 {
                format!("{}...", &content[..77])
            } else {
                content.to_string()
            };

            println!("{}. {}", idx + 1, content_preview);
            println!("   URL: {}", url);
            println!("   Published: {}", published);
            println!();
        }
    } else {
        println!("No posts found or unexpected response format.");
        println!("Response: {}", serde_json::to_string_pretty(&data)?);
    }

    Ok(())
}

pub async fn cmd_list_media(limit: usize) -> Result<()> {
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

    // Query for media using the source query
    let client = HttpClient::new();
    let response = client
        .get(format!(
            "{}?q=source&limit={}&filter=photo",
            micropub_endpoint, limit
        ))
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

    // The response format can vary, but typically has "items" array
    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        if items.is_empty() {
            println!("No media files found.");
            return Ok(());
        }

        println!("Recent media uploads:");
        println!();

        for (idx, item) in items.iter().enumerate() {
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
                .unwrap_or("(no URL)");

            // Get published date
            let published = properties
                .get("published")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no date)");

            // Get name/alt text if available
            let name = properties
                .get("name")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            println!("{}. {}", idx + 1, url);
            if let Some(n) = name {
                println!("   Name: {}", n);
            }
            println!("   Uploaded: {}", published);
            println!();
        }
    } else {
        println!("No media files found or unexpected response format.");
        println!("Your server may not support media queries.");
    }

    Ok(())
}
