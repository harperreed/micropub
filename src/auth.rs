// ABOUTME: Authentication and OAuth flow handling
// ABOUTME: Performs IndieAuth discovery and token management

use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use scraper::{Html, Selector};
use std::fs;
use url::Url;

use crate::config::{Config, Profile, get_tokens_dir};

// OAuth imports for future full OAuth implementation
#[allow(unused_imports)]
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
#[allow(unused_imports)]
use oauth2::basic::BasicClient;

/// Discover endpoints from a domain
async fn discover_endpoints(domain: &str) -> Result<(String, String, String)> {
    // Enforce HTTPS for security
    let url = if domain.starts_with("https://") {
        domain.to_string()
    } else if domain.starts_with("http://") {
        anyhow::bail!(
            "Insecure HTTP not allowed. Please use HTTPS: {}",
            domain.replace("http://", "https://")
        );
    } else {
        format!("https://{}", domain)
    };

    let client = HttpClient::new();
    let response = client.get(&url).send().await?;

    let mut micropub_endpoint = None;
    let mut authorization_endpoint = None;
    let mut token_endpoint = None;

    // First, check HTTP Link headers (preferred by spec)
    for link_header in response.headers().get_all("link") {
        if let Ok(link_str) = link_header.to_str() {
            // Parse Link header format: <url>; rel="relation"
            for link in link_str.split(',') {
                let parts: Vec<&str> = link.split(';').collect();
                if parts.len() < 2 {
                    continue;
                }

                // Extract URL (remove < and >)
                let url_part = parts[0].trim();
                let endpoint_url = url_part.trim_start_matches('<').trim_end_matches('>');

                // Extract rel value
                for param in &parts[1..] {
                    if let Some(rel_value) = param.trim().strip_prefix("rel=") {
                        let rel = rel_value.trim_matches('"').trim_matches('\'');

                        let resolved = resolve_url(&url, endpoint_url)?;
                        match rel {
                            "micropub" => micropub_endpoint = Some(resolved),
                            "authorization_endpoint" => authorization_endpoint = Some(resolved),
                            "token_endpoint" => token_endpoint = Some(resolved),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let html = response.text().await?;

    // Then check HTML <link> tags (fallback)
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("link[rel]").unwrap();

    for element in document.select(&link_selector) {
        let rel = element.value().attr("rel");
        let href = element.value().attr("href");

        match (rel, href) {
            (Some("micropub"), Some(href)) if micropub_endpoint.is_none() => {
                micropub_endpoint = Some(resolve_url(&url, href)?);
            }
            (Some("authorization_endpoint"), Some(href)) if authorization_endpoint.is_none() => {
                authorization_endpoint = Some(resolve_url(&url, href)?);
            }
            (Some("token_endpoint"), Some(href)) if token_endpoint.is_none() => {
                token_endpoint = Some(resolve_url(&url, href)?);
            }
            _ => {}
        }
    }

    let micropub = micropub_endpoint
        .context("Could not find micropub endpoint in Link headers or HTML")?;
    let auth = authorization_endpoint
        .context("Could not find authorization_endpoint in Link headers or HTML")?;
    let token = token_endpoint
        .context("Could not find token_endpoint in Link headers or HTML")?;

    Ok((micropub, auth, token))
}

/// Resolve a potentially relative URL
fn resolve_url(base: &str, href: &str) -> Result<String> {
    let base_url = Url::parse(base)?;
    let resolved = base_url.join(href)?;
    Ok(resolved.to_string())
}

/// Discover media endpoint from micropub endpoint
async fn discover_media_endpoint(micropub_endpoint: &str, token: &str) -> Result<Option<String>> {
    let client = HttpClient::new();
    let response = client
        .get(format!("{}?q=config", micropub_endpoint))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if response.status().is_success() {
        let config: serde_json::Value = response.json().await?;
        if let Some(media) = config.get("media-endpoint") {
            if let Some(media_str) = media.as_str() {
                return Ok(Some(media_str.to_string()));
            }
        }
    }

    Ok(None)
}

/// Perform OAuth authentication flow
pub async fn cmd_auth(domain: &str) -> Result<()> {
    println!("Discovering endpoints for {}...", domain);

    let (micropub_endpoint, auth_endpoint, token_endpoint) = discover_endpoints(domain).await?;

    println!("✓ Found micropub endpoint: {}", micropub_endpoint);
    println!("✓ Found authorization endpoint: {}", auth_endpoint);
    println!("✓ Found token endpoint: {}", token_endpoint);

    // For now, use manual token flow
    // TODO: Implement full OAuth flow with PKCE

    println!("\nManual token configuration:");
    println!("1. Visit your micropub provider's settings");
    println!("2. Generate an API token with 'create' scope");
    println!("3. Enter the token below");
    println!();
    print!("Token: ");

    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();
    if token.is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    // Discover media endpoint
    println!("\nDiscovering media endpoint...");
    let media_endpoint = discover_media_endpoint(&micropub_endpoint, &token).await?;

    if let Some(ref media) = media_endpoint {
        println!("✓ Found media endpoint: {}", media);
    } else {
        println!("⚠ No media endpoint found");
    }

    // Save profile
    let mut config = Config::load()?;

    let profile_name = domain.replace("https://", "").replace("http://", "");

    config.upsert_profile(
        profile_name.clone(),
        Profile {
            domain: domain.to_string(),
            micropub_endpoint: Some(micropub_endpoint),
            media_endpoint,
            token_endpoint: Some(token_endpoint),
            authorization_endpoint: Some(auth_endpoint),
        },
    );

    if config.default_profile.is_empty() {
        config.default_profile = profile_name.clone();
    }

    config.save()?;

    // Save token
    let token_path = get_tokens_dir()?.join(format!("{}.token", profile_name));
    fs::write(&token_path, token)?;

    // Set restrictive permissions on token file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&token_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&token_path, perms)?;
    }

    println!("\n✓ Authentication configured for profile: {}", profile_name);

    Ok(())
}