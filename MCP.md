# MCP Server Integration Status

## Goal
Add Model Context Protocol (MCP) server support to enable AI assistants to post directly to micropub endpoints.

## Current Status: BLOCKED - SDK Issues

### What's Done
- ✅ Added rmcp dependencies (v0.8.5) to Cargo.toml
- ✅ Created MCP module structure (`src/mcp.rs`)
- ✅ Added `micropub mcp` CLI command
- ✅ Designed 6 core tools:
  - `publish_post` - Create and publish immediately
  - `create_draft` - Save draft for later
  - `list_drafts` - View all drafts
  - `publish_backdate` - Publish with past timestamp
  - `delete_post` - Remove published post
  - `whoami` - Check authentication status

### Current Blocker
The `#[tool_router]` macro from rmcp has persistent compilation issues in both v0.8.5 and v0.9.1. The macro expects generated `_tool_attr` methods that aren't being created properly.

**Error pattern (both versions):**
```
error[E0599]: no function or associated item named `publish_post_tool_attr` found
error: cannot find attribute `tool` in this scope
```

**Tested versions:**
- ❌ rmcp v0.8.5 - Same macro generation failures
- ❌ rmcp v0.9.1 - Same macro generation failures

This appears to be a fundamental issue with the rmcp SDK's procedural macros. The SDK may be in an unstable state or have undocumented requirements.

## Next Steps

### Option 1: Manual JSON-RPC Implementation (Recommended)
- Implement manual MCP protocol handling
- Use stdio JSON-RPC directly
- Reference: https://modelcontextprotocol.io/docs/concepts/architecture

### Option 3: Wait for SDK Stabilization
The Rust SDK is still evolving. May need to wait for a more stable release or better documentation.

## MCP Server Design

Once working, the server will:

1. **Run via stdio**: `micropub mcp`
2. **Tools available**:
   - Publish posts directly from AI conversations
   - Create drafts for review
   - Backdate posts (great for importing old content)
   - List and manage drafts
   - Delete posts
   - Check auth status

3. **Configuration**:
   Uses existing micropub auth (no separate setup needed)

## Usage Example (when working)

In Claude Desktop config:
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

Then in Claude:
```
User: "Post this to my blog: Just had an amazing coffee at the new cafe downtown!"
Claude: *uses publish_post tool* "Posted to your micropub endpoint!"
```

## Testing Plan

Once compilation issues are resolved:

1. Test with MCP Inspector: `npx @modelcontextprotocol/inspector`
2. Test tool discovery
3. Test each tool individually
4. Integration test with Claude Desktop
5. Add photo upload support to publish_post

## Resources

- MCP Rust SDK: https://github.com/modelcontextprotocol/rust-sdk
- MCP Spec: https://modelcontextprotocol.io/
- rmcp docs: https://docs.rs/rmcp/

## Code Location

- MCP module: `src/mcp.rs`
- CLI command: `src/main.rs` (Commands::Mcp)
- Dependencies: `Cargo.toml`
