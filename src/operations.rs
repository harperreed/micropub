// ABOUTME: Post management operations (update, delete, undelete, whoami, list)
// ABOUTME: Handles modifications to existing posts and queries

use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{Map, Value};

use crate::client::{MicropubClient, MicropubRequest, MicropubAction};
use crate::config::{Config, load_token};

pub async fn cmd_update(url: &str) -> Result<()> {
    println!("Update operation not yet implemented for: {}", url);
    println!("This will allow updating existing posts");
    Ok(())
}

pub async fn cmd_delete(post_url: &str) -> Result<()> {
    let config = Config::load()?;

    let profile_name = &config.default_profile;
    if profile_name.is_empty() {
        anyhow::bail!("No default profile set. Run 'micropub auth' first");
    }

    let profile = config.get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile.micropub_endpoint.as_ref()
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

    let profile = config.get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile.micropub_endpoint.as_ref()
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

    let profile = config.get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile.micropub_endpoint.as_ref()
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

    let profile = config.get_profile(profile_name)
        .context("Profile not found")?;

    let token = load_token(profile_name)?;

    let micropub_endpoint = profile.micropub_endpoint.as_ref()
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
        let body = response.text().await.unwrap_or_else(|_| String::from("<unable to read response>"));
        anyhow::bail!("Failed to list posts: HTTP {}\n{}", status, body);
    }

    let data: Value = response.json().await
        .context("Failed to parse response")?;

    // The response format can vary, but typically has "items" array
    if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
        if items.is_empty() {
            println!("No posts found.");
            return Ok(());
        }

        println!("Recent posts:");
        println!();

        for (idx, item) in items.iter().enumerate() {
            let properties = item.get("properties")
                .context("Missing properties in post")?;

            // Get URL
            let url = properties.get("url")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("(no URL)");

            // Get content or name
            let content = properties.get("content")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .or_else(|| {
                    properties.get("name")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("(no content)");

            // Get published date
            let published = properties.get("published")
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
