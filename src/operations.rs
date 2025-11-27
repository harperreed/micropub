// ABOUTME: Post management operations (update, delete, undelete)
// ABOUTME: Handles modifications to existing posts

use anyhow::{Context, Result};
use serde_json::Map;

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
