// ABOUTME: Micropub HTTP client for API interactions
// ABOUTME: Handles requests, responses, and endpoint communication

use anyhow::{Context, Result};
use reqwest::{header, Client as HttpClient};
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub enum MicropubAction {
    Create,
    Update {
        replace: Map<String, Value>,
        add: Map<String, Value>,
        delete: Vec<String>,
    },
    Delete,
    Undelete,
}

#[derive(Debug, Clone)]
pub struct MicropubRequest {
    pub action: MicropubAction,
    pub properties: Map<String, Value>,
    pub url: Option<String>,
}

impl MicropubRequest {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        let mut obj = serde_json::Map::new();

        match &self.action {
            MicropubAction::Create => {
                obj.insert(
                    "type".to_string(),
                    Value::Array(vec![Value::String("h-entry".to_string())]),
                );
                obj.insert(
                    "properties".to_string(),
                    Value::Object(self.properties.clone()),
                );
            }
            MicropubAction::Update {
                replace,
                add,
                delete,
            } => {
                obj.insert("action".to_string(), Value::String("update".to_string()));
                let url = self
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for update action"))?;
                obj.insert("url".to_string(), Value::String(url.clone()));

                if !replace.is_empty() {
                    obj.insert("replace".to_string(), Value::Object(replace.clone()));
                }
                if !add.is_empty() {
                    obj.insert("add".to_string(), Value::Object(add.clone()));
                }
                if !delete.is_empty() {
                    obj.insert(
                        "delete".to_string(),
                        Value::Array(delete.iter().map(|s| Value::String(s.clone())).collect()),
                    );
                }
            }
            MicropubAction::Delete => {
                obj.insert("action".to_string(), Value::String("delete".to_string()));
                let url = self
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for delete action"))?;
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
            MicropubAction::Undelete => {
                obj.insert("action".to_string(), Value::String("undelete".to_string()));
                let url = self
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for undelete action"))?;
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
        }

        serde_json::to_string_pretty(&obj).context("Failed to serialize request")
    }
}

#[derive(Debug, Deserialize)]
pub struct MicropubResponse {
    pub url: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub struct MicropubClient {
    http_client: HttpClient,
    endpoint: String,
    token: String,
}

impl MicropubClient {
    pub fn new(endpoint: String, token: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            endpoint,
            token,
        }
    }

    /// Send a micropub request
    pub async fn send(&self, request: &MicropubRequest) -> Result<MicropubResponse> {
        let json = request.to_json()?;

        let response = self
            .http_client
            .post(&self.endpoint)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(json)
            .send()
            .await
            .context("Failed to send request to micropub endpoint")?;

        let status = response.status();

        // Get Location header for successful creates
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response.text().await?;

        if status.is_success() {
            Ok(MicropubResponse {
                url: location,
                error: None,
                error_description: None,
            })
        } else {
            // Try to parse error response
            let error_response: Result<MicropubResponse, _> = serde_json::from_str(&body);

            let error_msg = if let Ok(err) = error_response {
                format_error_message(&err.error, &err.error_description)
            } else {
                format!("HTTP {}: {}", status, body)
            };

            anyhow::bail!(error_msg);
        }
    }
}

fn format_error_message(error: &Option<String>, description: &Option<String>) -> String {
    let error_code = error.as_deref().unwrap_or("unknown_error");
    let desc = description.as_deref().unwrap_or("No description provided");

    match error_code {
        "insufficient_scope" => {
            format!(
                "Insufficient permissions: {}\n\nRe-authenticate with: micropub auth <domain>",
                desc
            )
        }
        "invalid_request" => {
            format!(
                "Invalid request: {}\n\nCheck your draft format and try again",
                desc
            )
        }
        "unauthorized" => {
            format!(
                "Unauthorized: {}\n\nYour token may be expired. Re-authenticate with: micropub auth <domain>",
                desc
            )
        }
        _ => {
            format!("Micropub error ({}): {}", error_code, desc)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_request() {
        let req = MicropubRequest {
            action: MicropubAction::Delete,
            properties: Map::new(),
            url: Some("https://example.com/post/1".to_string()),
        };

        let json = req.to_json().unwrap();
        assert!(json.contains("delete"));
        assert!(json.contains("example.com"));
    }
}
