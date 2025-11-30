// ABOUTME: Minimal MCP server test to verify rmcp macro works
// ABOUTME: Used for debugging compilation issues

#![allow(dead_code)]

use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content},
    tool, tool_router, ErrorData as McpError,
};

#[derive(Clone)]
pub struct SimpleServer {
    tool_router: ToolRouter<SimpleServer>,
}

#[tool_router]
impl SimpleServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Simple test tool")]
    async fn test(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("hello")]))
    }
}

fn main() {
    let _server = SimpleServer::new();
    println!("Simple MCP server created successfully");
}
