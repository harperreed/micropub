// ABOUTME: Test of upstream counter.rs example from rust-sdk
// ABOUTME: Verifies if official example compiles with our rmcp version

#![allow(dead_code)]
use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_router, ErrorData as McpError,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Counter>,
}

#[tool_router]
impl Counter {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Get the current counter value")]
    async fn get_value(&self) -> Result<CallToolResult, McpError> {
        let counter = self.counter.lock().await;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }
}

fn main() {
    let _counter = Counter::new();
    println!("Counter created successfully");
}
