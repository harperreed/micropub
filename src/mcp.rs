// ABOUTME: Model Context Protocol (MCP) server implementation
// ABOUTME: Provides tools for AI assistants to post and manage micropub content

use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{CallToolResult, Content};
use rmcp::tool_router;
use rmcp::ErrorData as McpError;
use rmcp::Service;
use tokio::io::{stdin, stdout};

use crate::config::Config;
use crate::draft::Draft;
use crate::publish;

/// MCP server state
#[derive(Clone)]
pub struct MicropubMcp {
    tool_router: ToolRouter<Self>,
}

impl MicropubMcp {
    /// Create a new MCP server instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl MicropubMcp {
    /// Create and publish a post immediately
    #[tool(description = "Create and publish a micropub post with optional title and categories")]
    async fn publish_post(
        &self,
        content: String,
        title: Option<String>,
        categories: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        // Create a draft first
        let mut draft = Draft::new(uuid::Uuid::new_v4().to_string());
        draft.content = content;
        draft.metadata.name = title;

        // Parse categories as comma-separated
        if let Some(cats) = categories {
            draft.metadata.category = cats.split(',').map(|s| s.trim().to_string()).collect();
        }

        let draft_path = draft
            .save()
            .map_err(|e| McpError::internal(format!("Failed to save draft: {}", e)))?;

        // Publish it
        publish::cmd_publish(draft_path.to_str().unwrap(), None)
            .await
            .map_err(|e| McpError::internal(format!("Failed to publish: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(
            "Post published successfully!",
        )]))
    }

    /// Create a draft post without publishing
    #[tool(description = "Create a draft micropub post for later editing and publishing")]
    async fn create_draft(
        &self,
        content: String,
        title: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let mut draft = Draft::new(uuid::Uuid::new_v4().to_string());
        draft.content = content;
        draft.metadata.name = title;

        draft
            .save()
            .map_err(|e| McpError::internal(format!("Failed to create draft: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Draft created with ID: {}",
            draft.id
        ))]))
    }

    /// List all draft posts
    #[tool(description = "List all draft micropub posts")]
    async fn list_drafts(&self) -> Result<CallToolResult, McpError> {
        let draft_ids = Draft::list_all()
            .map_err(|e| McpError::internal(format!("Failed to list drafts: {}", e)))?;

        if draft_ids.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No drafts found.",
            )]));
        }

        let mut output = String::from("Drafts:\n");
        for id in draft_ids {
            if let Ok(draft) = Draft::load(&id) {
                let title = draft
                    .metadata
                    .name
                    .unwrap_or_else(|| "[untitled]".to_string());
                output.push_str(&format!("- {} ({})\n", title, id));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Publish a draft with a backdated timestamp
    #[tool(description = "Publish a draft post with a specific past date (ISO 8601 format)")]
    async fn publish_backdate(
        &self,
        draft_id: String,
        date: String,
    ) -> Result<CallToolResult, McpError> {
        // Parse the date
        let parsed_date = DateTime::parse_from_rfc3339(&date)
            .map_err(|e| {
                McpError::invalid_params(format!(
                    "Invalid date format: {}. Use ISO 8601 like 2024-01-15T10:30:00Z",
                    e
                ))
            })?
            .with_timezone(&Utc);

        // Load draft path
        let draft_path = crate::config::get_drafts_dir()
            .map_err(|e| McpError::internal(format!("Failed to get drafts dir: {}", e)))?
            .join(format!("{}.md", draft_id));

        if !draft_path.exists() {
            return Err(McpError::invalid_params(format!(
                "Draft not found: {}",
                draft_id
            )));
        }

        // Publish with backdate
        publish::cmd_publish(draft_path.to_str().unwrap(), Some(parsed_date))
            .await
            .map_err(|e| McpError::internal(format!("Failed to publish: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Post published with backdated timestamp: {}",
            date
        ))]))
    }

    /// Delete a published post
    #[tool(description = "Delete a published micropub post by URL")]
    async fn delete_post(&self, url: String) -> Result<CallToolResult, McpError> {
        crate::operations::cmd_delete(&url)
            .await
            .map_err(|e| McpError::internal(format!("Failed to delete post: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Post deleted: {}",
            url
        ))]))
    }

    /// Get authentication status
    #[tool(description = "Check which micropub account is currently authenticated")]
    async fn whoami(&self) -> Result<CallToolResult, McpError> {
        let config = Config::load()
            .map_err(|e| McpError::internal(format!("Failed to load config: {}", e)))?;

        let profile_name = &config.default_profile;
        if profile_name.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No profile configured. Run 'micropub auth <domain>' first.",
            )]));
        }

        let profile = config
            .get_profile(profile_name)
            .ok_or_else(|| McpError::internal("Profile not found".to_string()))?;

        let output = format!(
            "Authenticated as:\n  Profile: {}\n  Domain: {}\n  Micropub: {}",
            profile_name,
            profile.domain,
            profile
                .micropub_endpoint
                .as_deref()
                .unwrap_or("(not configured)")
        );

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

/// Run the MCP server
pub async fn run_server() -> Result<()> {
    let server = MicropubMcp::new()?;

    eprintln!("Starting Micropub MCP server...");
    eprintln!("Ready to receive requests via stdio");

    // Create stdio transport
    let transport = (stdin(), stdout());

    // Start serving
    let service = server.serve(transport).await?;

    // Wait for shutdown
    service.waiting().await?;

    Ok(())
}
