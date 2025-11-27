// ABOUTME: Media file handling and upload functionality
// ABOUTME: Detects local file references, uploads to media endpoint, replaces URLs

use anyhow::{Context, Result};
use regex::Regex;
use reqwest::{Client as HttpClient, header, multipart};
use std::fs;
use std::path::{Path, PathBuf};

/// Find all media file references in content
pub fn find_media_references(content: &str) -> Vec<String> {
    let mut refs = Vec::new();

    // Markdown images: ![alt](path)
    let md_img_re = Regex::new(r"!\[.*?\]\((.*?)\)").unwrap();
    for cap in md_img_re.captures_iter(content) {
        if let Some(path) = cap.get(1) {
            let path_str = path.as_str();
            if is_local_path(path_str) {
                refs.push(path_str.to_string());
            }
        }
    }

    // HTML img tags: <img src="path">
    let html_img_re = Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#).unwrap();
    for cap in html_img_re.captures_iter(content) {
        if let Some(path) = cap.get(1) {
            let path_str = path.as_str();
            if is_local_path(path_str) {
                refs.push(path_str.to_string());
            }
        }
    }

    refs
}

/// Check if a path is local (not a URL)
fn is_local_path(path: &str) -> bool {
    !path.starts_with("http://") && !path.starts_with("https://")
}

/// Resolve a path (expand ~, handle relative paths)
pub fn resolve_path(path: &str, base_dir: Option<&Path>) -> Result<PathBuf> {
    let expanded = if path.starts_with("~/") {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;
        home.join(&path[2..])
    } else if path.starts_with('/') {
        PathBuf::from(path)
    } else if let Some(base) = base_dir {
        base.join(path)
    } else {
        PathBuf::from(path)
    };

    Ok(expanded)
}

/// Upload a file to media endpoint
pub async fn upload_file(
    endpoint: &str,
    token: &str,
    file_path: &Path,
) -> Result<String> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;

    let mime_type = mime_guess::from_path(file_path)
        .first_or_octet_stream();

    let file_bytes = fs::read(file_path)
        .context("Failed to read file")?;

    let part = multipart::Part::bytes(file_bytes)
        .file_name(filename.to_string())
        .mime_str(mime_type.as_ref())?;

    let form = multipart::Form::new()
        .part("file", part);

    let client = HttpClient::new();
    let response = client
        .post(endpoint)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .context("Failed to upload file")?;

    if !response.status().is_success() {
        anyhow::bail!("Upload failed: {}", response.status());
    }

    // Get URL from Location header
    let url = response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .context("No Location header in response")?
        .to_string();

    Ok(url)
}

/// Replace local paths in content with URLs
pub fn replace_paths(content: &str, replacements: &[(String, String)]) -> String {
    let mut result = content.to_string();

    for (local_path, url) in replacements {
        result = result.replace(local_path, url);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_local_path() {
        assert!(is_local_path("~/photo.jpg"));
        assert!(is_local_path("/abs/path.jpg"));
        assert!(is_local_path("relative/path.jpg"));
        assert!(!is_local_path("https://example.com/image.jpg"));
    }

    #[test]
    fn test_replace_paths() {
        let content = "Image: ![](~/photo.jpg) here";
        let replacements = vec![
            ("~/photo.jpg".to_string(), "https://cdn.com/abc.jpg".to_string())
        ];

        let result = replace_paths(content, &replacements);
        assert!(result.contains("https://cdn.com/abc.jpg"));
        assert!(!result.contains("~/photo.jpg"));
    }
}