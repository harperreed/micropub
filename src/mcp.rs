// ABOUTME: Model Context Protocol (MCP) server implementation
// ABOUTME: Provides tools for AI assistants to post and manage micropub content

use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, Implementation, ProtocolVersion, ServerCapabilities,
    ServerInfo,
};
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use rmcp::transport::stdio;
use rmcp::ErrorData as McpError;
use rmcp::{schemars, ServerHandler, ServiceExt};

use crate::config::Config;
use crate::draft::Draft;
use crate::publish;

/// Parameters for publish_post tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PublishPostArgs {
    /// The content of the post
    pub content: String,
    /// Optional title for the post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional comma-separated categories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<String>,
}

/// Parameters for create_draft tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateDraftArgs {
    /// The content of the draft
    pub content: String,
    /// Optional title for the draft
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Parameters for publish_backdate tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PublishBackdateArgs {
    /// The draft ID to publish (alphanumeric, hyphens, underscores only)
    #[schemars(regex(pattern = r"^[a-zA-Z0-9_-]+$"))]
    pub draft_id: String,
    /// ISO 8601 formatted date (e.g., 2024-01-15T10:30:00Z)
    pub date: String,
}

/// Parameters for delete_post tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeletePostArgs {
    /// The URL of the post to delete
    #[schemars(url)]
    pub url: String,
}

/// Parameters for list_posts tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListPostsArgs {
    /// Number of posts to retrieve (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,
}

/// Parameters for view_draft tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ViewDraftArgs {
    /// The draft ID to view
    #[schemars(regex(pattern = r"^[a-zA-Z0-9_-]+$"))]
    pub draft_id: String,
}

/// Parameters for list_media tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListMediaArgs {
    /// Number of media items to retrieve (default: 20)
    #[serde(default = "default_media_limit")]
    pub limit: usize,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    10
}

fn default_media_limit() -> usize {
    20
}

/// MCP server state
#[derive(Clone)]
pub struct MicropubMcp {
    tool_router: ToolRouter<MicropubMcp>,
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
        Parameters(args): Parameters<PublishPostArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate content is not empty
        if args.content.trim().is_empty() {
            return Err(McpError::invalid_params(
                "Content cannot be empty".to_string(),
                None,
            ));
        }

        // Create a draft first
        let mut draft = Draft::new(uuid::Uuid::new_v4().to_string());
        draft.content = args.content;
        draft.metadata.name = args.title;

        // Parse categories as comma-separated
        if let Some(cats) = args.categories {
            draft.metadata.category = cats.split(',').map(|s| s.trim().to_string()).collect();
        }

        let draft_path = draft.save().map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to save draft: {}", e),
                None,
            )
        })?;

        // Publish it
        let draft_path_str = draft_path.to_str().ok_or_else(|| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                "Draft path contains invalid UTF-8".to_string(),
                None,
            )
        })?;

        publish::cmd_publish(draft_path_str, None)
            .await
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to publish: {}", e),
                    None,
                )
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            "Post published successfully!",
        )]))
    }

    /// Create a draft post without publishing
    #[tool(description = "Create a draft micropub post for later editing and publishing")]
    async fn create_draft(
        &self,
        Parameters(args): Parameters<CreateDraftArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate content is not empty
        if args.content.trim().is_empty() {
            return Err(McpError::invalid_params(
                "Content cannot be empty".to_string(),
                None,
            ));
        }

        let mut draft = Draft::new(uuid::Uuid::new_v4().to_string());
        draft.content = args.content;
        draft.metadata.name = args.title;

        draft.save().map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create draft: {}", e),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Draft created with ID: {}",
            draft.id
        ))]))
    }

    /// List all draft posts
    #[tool(description = "List all draft micropub posts")]
    async fn list_drafts(&self) -> Result<CallToolResult, McpError> {
        let draft_ids = Draft::list_all().map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to list drafts: {}", e),
                None,
            )
        })?;

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
        Parameters(args): Parameters<PublishBackdateArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate draft_id format to prevent path traversal
        if args.draft_id.is_empty() {
            return Err(McpError::invalid_params(
                "Draft ID cannot be empty".to_string(),
                None,
            ));
        }
        if !args
            .draft_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(McpError::invalid_params(
                "Draft ID must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
                None,
            ));
        }

        // Parse the date
        let parsed_date = DateTime::parse_from_rfc3339(&args.date)
            .map_err(|e| {
                McpError::invalid_params(
                    format!(
                        "Invalid date format: {}. Use ISO 8601 like 2024-01-15T10:30:00Z",
                        e
                    ),
                    None,
                )
            })?
            .with_timezone(&Utc);

        // Load draft path
        let draft_path = crate::config::get_drafts_dir()
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to get drafts dir: {}", e),
                    None,
                )
            })?
            .join(format!("{}.md", args.draft_id));

        if !draft_path.exists() {
            return Err(McpError::invalid_params(
                format!("Draft not found: {}", args.draft_id),
                None,
            ));
        }

        // Publish with backdate
        let draft_path_str = draft_path.to_str().ok_or_else(|| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                "Draft path contains invalid UTF-8".to_string(),
                None,
            )
        })?;

        publish::cmd_publish(draft_path_str, Some(parsed_date))
            .await
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to publish: {}", e),
                    None,
                )
            })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Post published with backdated timestamp: {}",
            args.date
        ))]))
    }

    /// Delete a published post
    #[tool(description = "Delete a published micropub post by URL")]
    async fn delete_post(
        &self,
        Parameters(args): Parameters<DeletePostArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate URL is not empty
        if args.url.is_empty() {
            return Err(McpError::invalid_params(
                "URL cannot be empty".to_string(),
                None,
            ));
        }

        crate::operations::cmd_delete(&args.url)
            .await
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to delete post: {}", e),
                    None,
                )
            })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Post deleted: {}",
            args.url
        ))]))
    }

    /// Get authentication status
    #[tool(description = "Check which micropub account is currently authenticated")]
    async fn whoami(&self) -> Result<CallToolResult, McpError> {
        let config = Config::load().map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to load config: {}", e),
                None,
            )
        })?;

        let profile_name = &config.default_profile;
        if profile_name.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No profile configured. Run 'micropub auth <domain>' first.",
            )]));
        }

        let profile = config.get_profile(profile_name).ok_or_else(|| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                "Profile not found".to_string(),
                None,
            )
        })?;

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

    /// List published posts
    #[tool(description = "List published micropub posts with pagination")]
    async fn list_posts(
        &self,
        Parameters(args): Parameters<ListPostsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let posts = crate::operations::fetch_posts(args.limit, args.offset)
            .await
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to fetch posts: {}", e),
                    None,
                )
            })?;

        if posts.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No posts found.",
            )]));
        }

        let mut output = String::from("Posts:\n\n");
        for post in posts {
            let title = post.name.unwrap_or_else(|| "[untitled]".to_string());
            output.push_str(&format!("- {} ({})\n", title, post.url));
            output.push_str(&format!("  Published: {}\n", post.published));
            if !post.categories.is_empty() {
                output.push_str(&format!("  Categories: {}\n", post.categories.join(", ")));
            }
            if !post.content.is_empty() {
                let preview = if post.content.len() > 100 {
                    format!("{}...", &post.content[..100])
                } else {
                    post.content.clone()
                };
                output.push_str(&format!("  Preview: {}\n", preview));
            }
            output.push('\n');
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// View a specific draft
    #[tool(description = "View the content of a specific draft")]
    async fn view_draft(
        &self,
        Parameters(args): Parameters<ViewDraftArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate draft_id format to prevent path traversal
        if args.draft_id.is_empty() {
            return Err(McpError::invalid_params(
                "Draft ID cannot be empty".to_string(),
                None,
            ));
        }
        if !args
            .draft_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(McpError::invalid_params(
                "Draft ID must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
                None,
            ));
        }

        let draft = Draft::load(&args.draft_id)
            .map_err(|e| McpError::invalid_params(format!("Failed to load draft: {}", e), None))?;

        let mut output = String::new();
        output.push_str(&format!("Draft: {}\n\n", args.draft_id));

        if let Some(ref title) = draft.metadata.name {
            output.push_str(&format!("Title: {}\n", title));
        }
        if !draft.metadata.category.is_empty() {
            output.push_str(&format!(
                "Categories: {}\n",
                draft.metadata.category.join(", ")
            ));
        }
        output.push_str(&format!("\nContent:\n{}", draft.content));

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List media files
    #[tool(description = "List uploaded media files with pagination")]
    async fn list_media(
        &self,
        Parameters(args): Parameters<ListMediaArgs>,
    ) -> Result<CallToolResult, McpError> {
        let media_items = crate::operations::fetch_media(args.limit, args.offset)
            .await
            .map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to fetch media: {}", e),
                    None,
                )
            })?;

        if media_items.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No media files found.",
            )]));
        }

        let mut output = String::from("Media files:\n\n");
        for item in media_items {
            output.push_str(&format!("- {}\n", item.url));
            if let Some(ref name) = item.name {
                output.push_str(&format!("  Name: {}\n", name));
            }
            output.push_str(&format!("  Uploaded: {}\n\n", item.uploaded));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

/// Implement ServerHandler to provide server metadata
#[tool_handler]
impl ServerHandler for MicropubMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Micropub MCP server for posting and managing micropub content via AI assistants"
                    .to_string(),
            ),
        }
    }
}

/// Run the MCP server
pub async fn run_server() -> Result<()> {
    eprintln!("Starting Micropub MCP server...");
    eprintln!("Ready to receive requests via stdio");

    // Create server and serve via stdio
    let service = MicropubMcp::new()?.serve(stdio()).await?;

    // Wait for shutdown
    service.waiting().await?;

    Ok(())
}
