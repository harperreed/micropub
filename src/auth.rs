// ABOUTME: Authentication and OAuth flow handling
// ABOUTME: Performs IndieAuth discovery and token management with PKCE

use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use rand::Rng;
use reqwest::Client as HttpClient;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use url::Url;

use crate::config::{get_tokens_dir, Config, Profile};

/// Discover endpoints from a domain
async fn discover_endpoints(domain: &str) -> Result<(String, String, String)> {
    // Check if this is a localhost/development domain
    let is_localhost = domain.starts_with("localhost")
        || domain.starts_with("127.0.0.1")
        || domain.starts_with("::1")
        || domain.starts_with("[::1]")
        || domain.starts_with("http://localhost")
        || domain.starts_with("http://127.0.0.1")
        || domain.starts_with("http://::1")
        || domain.starts_with("http://[::1]")
        || domain.starts_with("https://localhost")
        || domain.starts_with("https://127.0.0.1")
        || domain.starts_with("https://::1")
        || domain.starts_with("https://[::1]");

    // Enforce HTTPS for security (except localhost for development)
    let url = if domain.starts_with("https://") {
        domain.to_string()
    } else if domain.starts_with("http://") {
        if is_localhost {
            domain.to_string() // Allow HTTP for localhost
        } else {
            anyhow::bail!(
                "Insecure HTTP not allowed. Please use HTTPS: {}",
                domain.replace("http://", "https://")
            );
        }
    } else {
        // No scheme provided
        if is_localhost {
            format!("http://{}", domain) // Default to HTTP for localhost
        } else {
            format!("https://{}", domain) // Default to HTTPS for remote
        }
    };

    let client = HttpClient::new();
    let response = client.get(&url).send().await?;

    // Use final URL after redirects for resolving relative links
    let final_url = response.url().to_string();

    // Validate that redirects didn't downgrade us from HTTPS to HTTP (except localhost)
    let final_url_parsed = Url::parse(&final_url)?;
    if final_url_parsed.scheme() != "https" && !is_localhost {
        anyhow::bail!(
            "Security error: Server redirected to insecure HTTP ({}). This is not allowed.",
            final_url
        );
    }

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

                        let resolved = resolve_url(&final_url, endpoint_url)?;
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
                micropub_endpoint = Some(resolve_url(&final_url, href)?);
            }
            (Some("authorization_endpoint"), Some(href)) if authorization_endpoint.is_none() => {
                authorization_endpoint = Some(resolve_url(&final_url, href)?);
            }
            (Some("token_endpoint"), Some(href)) if token_endpoint.is_none() => {
                token_endpoint = Some(resolve_url(&final_url, href)?);
            }
            _ => {}
        }
    }

    let micropub =
        micropub_endpoint.context("Could not find micropub endpoint in Link headers or HTML")?;
    let auth = authorization_endpoint
        .context("Could not find authorization_endpoint in Link headers or HTML")?;
    let token = token_endpoint.context("Could not find token_endpoint in Link headers or HTML")?;

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
        .await
        .context("Failed to query micropub config endpoint for media discovery")?;

    if response.status().is_success() {
        let config: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse micropub config response")?;
        if let Some(media) = config.get("media-endpoint") {
            if let Some(media_str) = media.as_str() {
                return Ok(Some(media_str.to_string()));
            }
        }
    }

    Ok(None)
}

/// Generate a cryptographically secure PKCE code verifier
fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    (0..128)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            match idx {
                0..=25 => (b'A' + idx) as char,
                26..=51 => (b'a' + (idx - 26)) as char,
                _ => (b'0' + (idx - 52)) as char,
            }
        })
        .collect()
}

/// Generate PKCE code challenge from verifier
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a random state parameter
fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    (0..32).map(|_| format!("{:x}", rng.gen::<u8>())).collect()
}

/// Struct to hold OAuth callback data
#[derive(Clone)]
struct OAuthCallback {
    code: Arc<Mutex<Option<String>>>,
    state: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
}

/// Handle OAuth callback from authorization server
async fn handle_callback(
    req: Request<Body>,
    callback_data: Arc<OAuthCallback>,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri();
    let query = uri.query().unwrap_or("");

    let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    if let Some(error) = params.get("error") {
        *callback_data.error.lock().unwrap() = Some(error.clone());

        let error_desc = params
            .get("error_description")
            .map(|s| s.as_str())
            .unwrap_or("Unknown error");

        let html = format!(
            r#"<html><body><h1>Authentication Failed</h1><p>Error: {}</p><p>{}</p><p>You can close this window.</p></body></html>"#,
            error, error_desc
        );

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .body(Body::from(html))
            .unwrap());
    }

    if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
        *callback_data.code.lock().unwrap() = Some(code.clone());
        *callback_data.state.lock().unwrap() = Some(state.clone());

        let html = r#"<html><body><h1>Authentication Successful!</h1><p>You can close this window and return to the terminal.</p><script>window.close();</script></body></html>"#;

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .body(Body::from(html))
            .unwrap());
    }

    let html =
        r#"<html><body><h1>Invalid Callback</h1><p>Missing required parameters.</p></body></html>"#;
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "text/html")
        .body(Body::from(html))
        .unwrap())
}

/// Find and bind to an available port from candidates
fn find_and_bind_port() -> Result<std::net::TcpListener> {
    let candidate_ports = [8089, 8090, 8091, 8092, 8093];

    for port in candidate_ports {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        if let Ok(listener) = std::net::TcpListener::bind(addr) {
            return Ok(listener);
        }
    }

    // Fallback: let OS choose a random available port
    println!("⚠ All preferred ports (8089-8093) occupied, using OS-assigned random port...");
    std::net::TcpListener::bind("127.0.0.1:0")
        .context("Failed to bind to any port, including OS-assigned random port")
}

/// Start local server to receive OAuth callback
async fn start_callback_server(
    callback_data: Arc<OAuthCallback>,
    listener: std::net::TcpListener,
) -> Result<()> {
    // Clone for shutdown signal before moving into make_svc
    let shutdown_signal = callback_data.clone();

    let make_svc = make_service_fn(move |_conn| {
        let callback_data = callback_data.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_callback(req, callback_data.clone())
            }))
        }
    });

    let server = Server::from_tcp(listener)?.serve(make_svc);

    // Run server with graceful shutdown
    let graceful = server.with_graceful_shutdown(async move {
        // Wait until we have a code or error
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if shutdown_signal.code.lock().unwrap().is_some()
                || shutdown_signal.error.lock().unwrap().is_some()
            {
                break;
            }
        }
    });

    tokio::select! {
        result = graceful => {
            result.context("Server error")?;
        },
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
            anyhow::bail!("OAuth callback timeout after 5 minutes");
        }
    }

    Ok(())
}

/// Exchange authorization code for access token
async fn exchange_code_for_token(
    token_endpoint: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
    client_id: &str,
) -> Result<String> {
    let client = HttpClient::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let response = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .context("Failed to exchange authorization code")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unable to read response>"));
        anyhow::bail!("Token exchange failed with status {}: {}", status, body);
    }

    let token_response: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse token response")?;

    token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("No access_token in response")
}

/// Validate OAuth scope contains only safe characters
fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty() {
        return Ok(()); // Empty scope is valid
    }

    // Allow alphanumeric, spaces, hyphens, and underscores only
    if !scope
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
    {
        anyhow::bail!(
            "Invalid scope '{}': only alphanumeric characters, spaces, hyphens, and underscores allowed",
            scope
        );
    }

    Ok(())
}

/// Perform OAuth authentication flow
pub async fn cmd_auth(domain: &str, scope: Option<&str>) -> Result<()> {
    // Load config to get client_id (if configured)
    let mut config = Config::load()?;

    println!("Discovering endpoints for {}...", domain);

    let (micropub_endpoint, auth_endpoint, token_endpoint) = discover_endpoints(domain).await?;

    println!("✓ Found micropub endpoint: {}", micropub_endpoint);
    println!("✓ Found authorization endpoint: {}", auth_endpoint);
    println!("✓ Found token endpoint: {}", token_endpoint);

    // Find and bind to an available port for the callback server
    let listener = find_and_bind_port()?;
    let port = listener.local_addr()?.port();
    println!("Using port {} for OAuth callback", port);

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();

    // Set up OAuth parameters
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);
    // Use configured client_id or default to GitHub repo URL
    let client_id = config
        .client_id
        .as_deref()
        .unwrap_or("https://github.com/harperreed/micropub");

    // Build authorization URL
    let scope = scope.unwrap_or("create update delete media");
    validate_scope(scope)?;

    // Build "me" parameter - must match the authenticated identity
    // Use the same scheme detection logic as endpoint discovery
    let me_param = if domain.starts_with("http://") || domain.starts_with("https://") {
        // Domain already has scheme, use as-is
        domain.to_string()
    } else {
        // No scheme - use http:// for localhost, https:// for remote
        let is_localhost = domain.starts_with("localhost")
            || domain.starts_with("127.0.0.1")
            || domain.starts_with("::1")
            || domain.starts_with("[::1]");

        if is_localhost {
            format!("http://{}", domain)
        } else {
            format!("https://{}", domain)
        }
    };

    let mut auth_url = Url::parse(&auth_endpoint)?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("scope", scope)
        .append_pair("me", &me_param);

    println!("\nStarting OAuth flow...");
    println!("Opening your browser to authenticate...");
    println!();

    // Set up callback receiver
    let callback_data = Arc::new(OAuthCallback {
        code: Arc::new(Mutex::new(None)),
        state: Arc::new(Mutex::new(None)),
        error: Arc::new(Mutex::new(None)),
    });

    // Start local callback server in background
    let callback_data_clone = callback_data.clone();
    let server_handle =
        tokio::spawn(async move { start_callback_server(callback_data_clone, listener).await });

    // Open browser
    if let Err(e) = open::that(auth_url.as_str()) {
        println!("⚠ Could not open browser automatically: {}", e);
        println!("Please open this URL manually:");
        println!("{}", auth_url);
    }

    println!("\nWaiting for authorization...");

    // Wait for the server to complete (it will shut down automatically after receiving callback)
    match server_handle.await {
        Ok(Ok(())) => {
            // Server completed successfully
        }
        Ok(Err(e)) => {
            anyhow::bail!("OAuth callback server error: {}", e);
        }
        Err(e) => {
            anyhow::bail!("OAuth server task panicked: {}", e);
        }
    }

    // Check for error
    if let Some(error) = callback_data.error.lock().unwrap().clone() {
        anyhow::bail!("Authorization failed: {}", error);
    }

    // Extract code and state
    let code = callback_data
        .code
        .lock()
        .unwrap()
        .clone()
        .context("No authorization code received")?;
    let received_state = callback_data
        .state
        .lock()
        .unwrap()
        .clone()
        .context("No state received")?;

    // Verify state matches
    if received_state != state {
        anyhow::bail!("State mismatch - possible CSRF attack");
    }

    println!("✓ Authorization code received");
    println!("\nExchanging code for access token...");

    // Exchange code for token
    let token = exchange_code_for_token(
        &token_endpoint,
        &code,
        &code_verifier,
        &redirect_uri,
        client_id,
    )
    .await?;

    println!("✓ Access token obtained");

    // Validate the token before saving it
    println!("\nValidating token...");
    let client = HttpClient::new();
    let validation_response = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        client
            .get(format!("{}?q=config", micropub_endpoint))
            .header("Authorization", format!("Bearer {}", token))
            .send(),
    )
    .await
    .context("Timeout validating token (10 seconds) - micropub endpoint did not respond")??;

    match validation_response.status() {
        // Success - token is valid
        status if status.is_success() => {
            println!("✓ Token validated");
        }
        // Token is actually invalid
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            anyhow::bail!(
                "Token validation failed - the token was rejected (status {}). The authorization server may have issued an invalid token.",
                validation_response.status()
            );
        }
        // Rate limited - token is probably valid, just can't verify right now
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            println!("⚠ Warning: Rate limited during token validation (status 429). Saving token anyway.");
            println!("  The token is likely valid but couldn't be verified due to rate limiting.");
        }
        // Server error - don't reject token due to temporary issues
        status if status.is_server_error() => {
            println!("⚠ Warning: Micropub endpoint returned server error (status {}). Saving token anyway.", status);
            println!("  The token is likely valid but couldn't be verified due to server issues.");
        }
        // Other client errors
        status => {
            let body = validation_response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<unable to read response>"));
            anyhow::bail!(
                "Token validation failed with unexpected status {}: {}",
                status,
                body
            );
        }
    }

    // Save profile and token AFTER validation succeeds
    // (config already loaded at start of function)
    let profile_name = if domain.starts_with("http://") || domain.starts_with("https://") {
        let parsed = Url::parse(domain)?;
        let host = parsed.host_str().context("Invalid domain: missing host")?;

        // Include port in profile name if present
        match parsed.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_string(),
        }
    } else {
        domain.to_string()
    };

    // Save token immediately after obtaining it
    let tokens_dir = get_tokens_dir()?;
    let token_path = tokens_dir.join(format!("{}.token", profile_name));
    fs::write(&token_path, &token)?;

    // Set restrictive permissions on token file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&token_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&token_path, perms)?;
    }

    println!("✓ Token saved");

    // Now discover media endpoint (non-fatal if it fails)
    println!("\nDiscovering media endpoint...");
    let media_endpoint = match discover_media_endpoint(&micropub_endpoint, &token).await {
        Ok(endpoint) => {
            if let Some(ref media) = endpoint {
                println!("✓ Found media endpoint: {}", media);
            } else {
                println!("⚠ No media endpoint found");
            }
            endpoint
        }
        Err(e) => {
            println!("⚠ Could not discover media endpoint: {}", e);
            None
        }
    };

    // Save profile configuration
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

    // Always set this profile as default when authenticating
    config.default_profile = profile_name.clone();

    config.save()?;

    println!(
        "\n✓ Authentication configured for profile: {}",
        profile_name
    );

    Ok(())
}
