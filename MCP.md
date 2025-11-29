# MCP Server Integration Status

## Goal
Add Model Context Protocol (MCP) server support to enable AI assistants to post directly to micropub endpoints.

## Current Status: ✅ WORKING

### What's Implemented
- ✅ Added rmcp dependencies (v0.9.1 with transport-io feature) to Cargo.toml
- ✅ Created MCP module structure (`src/mcp.rs`)
- ✅ Added `micropub mcp` CLI command
- ✅ Implemented 10 core tools:
  - `publish_post` - Create and publish immediately
  - `create_draft` - Save draft for later
  - `list_drafts` - View all drafts
  - `view_draft` - Read content of a specific draft
  - `publish_backdate` - Publish with past timestamp
  - `delete_post` - Remove published post
  - `list_posts` - View published posts with pagination
  - `list_media` - View uploaded media files
  - `whoami` - Check authentication status
- ✅ Full ServerHandler implementation with tool metadata
- ✅ Basic test coverage (7 passing tests)
- ✅ Compilation successful with rmcp v0.9.1
- ✅ Security hardened (input validation, path traversal protection, panic prevention)

### Key Implementation Details

**Critical Pattern**: Tool parameters MUST use the `Parameters<T>` wrapper pattern from rmcp.

**Correct pattern:**
```rust
use rmcp::handler::server::wrapper::Parameters;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PublishPostArgs {
    pub content: String,
    pub title: Option<String>,
}

#[tool(description = "Create and publish a post")]
async fn publish_post(
    &self,
    Parameters(args): Parameters<PublishPostArgs>,
) -> Result<CallToolResult, McpError> {
    // Implementation
}
```

**Wrong pattern (will not compile):**
```rust
async fn publish_post(&self, content: String, title: Option<String>) -> Result<...> {
    // This fails with IntoToolRoute trait bound errors
}
```

**Other requirements:**
- Tool router field must use concrete type: `ToolRouter<MicropubMcp>` not `ToolRouter<Self>`
- Import from `rmcp::handler::server::router::tool::ToolRouter` (not the shorter path)
- Use `Implementation::from_build_env()` for server metadata
- Enable `transport-io` feature in Cargo.toml for stdio support

### Security Features

All tools implement comprehensive input validation:

1. **Path Traversal Protection** - `draft_id` parameters are validated to allow only alphanumeric characters, hyphens, and underscores
2. **Empty Input Validation** - All required fields (content, url, draft_id) reject empty or whitespace-only values
3. **Panic Prevention** - All `.to_str()` calls on paths use `.ok_or_else()` instead of `.unwrap()` to prevent panics on non-UTF-8 paths
4. **Schema Validation** - Uses `schemars` decorators for client-side validation (regex patterns, URL format)
5. **Runtime Validation** - Server-side validation ensures malicious input cannot bypass schema checks

### Resolution of Previous SDK Issues

Previous documentation incorrectly identified rmcp v0.9.1 as having "fundamental macro issues". The actual problem was incorrect usage patterns:

1. **Not using Parameters wrapper** - The tool_router macro requires all parameters to be wrapped in `Parameters<T>`
2. **Wrong import paths** - Some imports were using abbreviated paths that caused trait resolution issues
3. **Using Self instead of concrete type** - The ToolRouter generic must be the concrete struct type

Once these patterns were corrected, the SDK works perfectly with rmcp v0.9.1 (released Nov 24, 2025).

## Running the MCP Server

### Start the server
```bash
micropub mcp
```

The server will:
1. Start listening on stdio
2. Wait for MCP protocol messages
3. Expose all 10 tools to connecting clients
4. Use existing micropub authentication (no separate setup)

### Configuration

In Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "micropub": {
      "command": "/path/to/micropub",
      "args": ["mcp"]
    }
  }
}
```

Replace `/path/to/micropub` with the full path to your micropub binary.

### Example Usage

Once connected to Claude Desktop:

```
User: "Post this to my blog: Just had an amazing coffee at the new cafe downtown!"
Claude: *uses publish_post tool* "Posted to your micropub endpoint!"

User: "Create a draft about my trip to Portland"
Claude: *uses create_draft tool* "Draft created with ID: abc-123. You can edit it later."

User: "List my drafts"
Claude: *uses list_drafts tool* "You have 3 drafts:
- Trip to Portland (abc-123)
- Coffee Review (def-456)
- ...

User: "Show me my recent posts"
Claude: *uses list_posts tool* "Here are your recent posts:
- Morning Coffee (https://example.com/2024/01/coffee)
- Portland Trip (https://example.com/2024/01/portland)
- ...

User: "What's in draft abc-123?"
Claude: *uses view_draft tool* "This draft is titled 'Trip to Portland' and contains...
```

## Testing

### Unit Tests
```bash
cargo test --test mcp_tests
```

All 7 tests pass:
- ✅ MCP server initialization
- ✅ publish_post tool
- ✅ create_draft tool
- ✅ list_drafts tool
- ✅ publish_backdate tool
- ✅ delete_post tool
- ✅ whoami tool

Note: Tests for list_posts, list_media, and view_draft are placeholders pending MCP test framework improvements.

### Integration Testing

Test with MCP Inspector:
```bash
npx @modelcontextprotocol/inspector micropub mcp
```

This will:
1. Connect to the MCP server
2. List available tools
3. Allow manual testing of each tool
4. Show JSON-RPC messages

### Test with Claude Desktop

1. Add configuration (see above)
2. Restart Claude Desktop
3. Start a conversation
4. Try using the tools naturally in conversation
5. Claude will automatically discover and use the micropub tools

## Future Enhancements

- [ ] Add photo upload support to publish_post tool
- [ ] Add resource support (expose published posts as MCP resources)
- [ ] Add prompt support (templates for common post types)
- [ ] Integration tests with actual MCP clients
- [ ] Support for multiple micropub endpoints/profiles
- [ ] Draft editing via MCP
- [ ] Post search/filtering tools

## Resources

- MCP Rust SDK: https://github.com/modelcontextprotocol/rust-sdk
- MCP Spec: https://modelcontextprotocol.io/
- rmcp docs: https://docs.rs/rmcp/
- MCP Inspector: https://github.com/modelcontextprotocol/inspector

## Code Location

- MCP module: `src/mcp.rs` (~530 lines)
- Parameter types: Lines 23-100
- Tool implementations: Lines 118-510
- ServerHandler: Lines 513-525
- CLI command: `src/main.rs:89` (Commands::Mcp)
- Dependencies: `Cargo.toml:36-37`
- Tests: `tests/mcp_tests.rs`

## Troubleshooting

### "IntoToolRoute trait bound not satisfied"
You're not using the Parameters wrapper. All tool parameters must use `Parameters<YourType>`.

### "cannot find attribute `tool`"
Missing import: add `use rmcp::tool;`

### "module `mcp` is private"
Module not enabled in lib.rs. Add `pub mod mcp;`

### Server starts but Claude can't see tools
1. Check Claude Desktop config path is correct
2. Restart Claude Desktop after config changes
3. Check server stderr for initialization messages
4. Verify micropub auth is configured (`micropub whoami`)
