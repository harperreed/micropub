// ABOUTME: Micropub HTTP client for API interactions
// ABOUTME: Handles requests, responses, and endpoint communication

use anyhow::{Context, Result};
use reqwest::{Client as HttpClient, header};
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
                obj.insert("type".to_string(), Value::Array(vec![Value::String("h-entry".to_string())]));
                obj.insert("properties".to_string(), Value::Object(self.properties.clone()));
            }
            MicropubAction::Update { replace, add, delete } => {
                obj.insert("action".to_string(), Value::String("update".to_string()));
                obj.insert("url".to_string(), Value::String(self.url.clone().unwrap_or_default()));

                if !replace.is_empty() {
                    obj.insert("replace".to_string(), Value::Object(replace.clone()));
                }
                if !add.is_empty() {
                    obj.insert("add".to_string(), Value::Object(add.clone()));
                }
                if !delete.is_empty() {
                    obj.insert("delete".to_string(), Value::Array(
                        delete.iter().map(|s| Value::String(s.clone())).collect()
                    ));
                }
            }
            MicropubAction::Delete => {
                obj.insert("action".to_string(), Value::String("delete".to_string()));
                obj.insert("url".to_string(), Value::String(self.url.clone().unwrap_or_default()));
            }
            MicropubAction::Undelete => {
                obj.insert("action".to_string(), Value::String("undelete".to_string()));
                obj.insert("url".to_string(), Value::String(self.url.clone().unwrap_or_default()));
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

        let response = self.http_client
            .post(&self.endpoint)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(json)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();

        // Get Location header for successful creates
        let location = response.headers()
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
            let error_response: MicropubResponse = serde_json::from_str(&body)
                .unwrap_or(MicropubResponse {
                    url: None,
                    error: Some("unknown_error".to_string()),
                    error_description: Some(body),
                });

            anyhow::bail!(
                "Micropub error: {} - {}",
                error_response.error.unwrap_or_default(),
                error_response.error_description.unwrap_or_default()
            );
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
