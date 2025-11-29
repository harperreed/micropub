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
- ✅ Implemented 6 workflow prompts:
  - `quick-note` - Post a quick note or thought
  - `photo-post` - Create a photo post with caption
  - `article-draft` - Create a longer article draft
  - `backdate-memory` - Record a memory with past date
  - `categorized-post` - Create post with categories
  - `new-post` - General posting workflow guide
- ✅ Full ServerHandler implementation with tool and prompt metadata
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

**Prompt pattern (same Parameters<T> requirement):**
```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QuickNotePromptArgs {
    pub topic: String,
}

#[prompt(
    name = "quick-note",
    description = "Post a quick note or thought"
)]
async fn quick_note(&self, Parameters(args): Parameters<QuickNotePromptArgs>) -> GetPromptResult {
    GetPromptResult {
        description: Some("Quick note posting workflow".to_string()),
        messages: vec![
            PromptMessage::new_text(PromptMessageRole::User, format!("...", args.topic)),
            PromptMessage::new_text(PromptMessageRole::Assistant, "...".to_string()),
        ],
    }
}
```

**Other requirements:**
- Tool router field must use concrete type: `ToolRouter<MicropubMcp>` not `ToolRouter<Self>`
- Prompt router field must use concrete type: `PromptRouter<MicropubMcp>` not `PromptRouter<Self>`
- Import from `rmcp::handler::server::router::tool::ToolRouter` (not the shorter path)
- Import from `rmcp::handler::server::router::prompt::PromptRouter`
- Use `Implementation::from_build_env()` for server metadata
- Enable `transport-io` feature in Cargo.toml for stdio support
- Enable both tools and prompts in capabilities: `.enable_tools().enable_prompts()`
- Add `#[prompt_handler(router = self.prompt_router)]` to ServerHandler impl

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

### Using Prompts

The MCP server provides workflow prompts that guide AI assistants through common micropub posting patterns. Prompts help structure conversations and provide context-specific guidance.

**Available Prompts:**

1. **quick-note** - Post a quick note or thought
   ```
   User: [selects quick-note prompt, enters topic: "morning coffee"]
   Claude: "I'll help you create a quick note about morning coffee. What would you like to say?"
   ```

2. **photo-post** - Create a photo post with caption
   ```
   User: [selects photo-post prompt, enters subject: "sunset at the beach"]
   Claude: "I'll help you create a photo post about sunset at the beach. Please provide:
   1. The photo file path or URL
   2. A caption for the photo
   3. Any additional context or description"
   ```

3. **article-draft** - Create a longer article draft
   ```
   User: [selects article-draft prompt, enters topic: "Rust async patterns", key_points: "tokio, async/await, error handling"]
   Claude: "I'll help you draft an article about Rust async patterns. Let's start with:
   1. A compelling title
   2. An introduction that hooks the reader
   3. Main body sections covering your key points
   4. A conclusion

   This will be saved as a draft for you to edit before publishing."
   ```

4. **backdate-memory** - Record a memory with past date
   ```
   User: [selects backdate-memory prompt, memory: "graduated from college", when: "May 2015"]
   Claude: "I'll help you record this memory from May 2015. Let's:
   1. Write out the full memory in detail
   2. Convert 'May 2015' to a specific date (ISO 8601 format)
   3. Save it as a draft
   4. Publish it with the backdated timestamp

   Tell me more about what happened."
   ```

5. **categorized-post** - Create post with specific categories
   ```
   User: [selects categorized-post prompt, topic: "new project launch", categories: "projects,announcements"]
   Claude: "I'll help you create a post about new project launch with categories: projects, announcements.

   What would you like to say? I'll make sure to tag it appropriately."
   ```

6. **new-post** - General posting workflow guide
   ```
   User: [selects new-post prompt]
   Claude: "I'll help you create a new micropub post! What type of post would you like to make?

   - Quick note or thought
   - Photo with caption
   - Longer article (saved as draft)
   - Backdated memory
   - Categorized post

   Or just tell me what you want to post and I'll figure out the best format!"
   ```

**How Prompts Work:**

- Prompts are templates that pre-fill conversation context
- They guide the AI assistant through structured workflows
- They can accept arguments to customize the workflow
- They're user-initiated (typically via UI selection in Claude Desktop)
- They provide a more natural, conversational way to use the MCP tools

**In Claude Desktop:**

Prompts appear as suggested workflows that users can select. When you select a prompt, Claude Desktop may ask you to fill in any required parameters (like "topic" or "subject"), then starts the conversation with the pre-filled context.
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
- [ ] Integration tests with actual MCP clients
- [ ] Support for multiple micropub endpoints/profiles
- [ ] Draft editing via MCP
- [ ] Post search/filtering tools
- [ ] More specialized prompts (book reviews, trip reports, etc.)

## Resources

- MCP Rust SDK: https://github.com/modelcontextprotocol/rust-sdk
- MCP Spec: https://modelcontextprotocol.io/
- rmcp docs: https://docs.rs/rmcp/
- MCP Inspector: https://github.com/modelcontextprotocol/inspector

## Code Location

- MCP module: `src/mcp.rs` (~760 lines)
- Parameter types: Lines 28-146
- Tool implementations: Lines 165-558
- Prompt implementations: Lines 560-733
- ServerHandler: Lines 735-753
- CLI command: `src/main.rs:89` (Commands::Mcp)
- Dependencies: `Cargo.toml:36-37`
- Tests: `tests/mcp_tests.rs`

## Troubleshooting

### "IntoToolRoute trait bound not satisfied"
You're not using the Parameters wrapper. All tool parameters must use `Parameters<YourType>`.

### "cannot find attribute `tool`"
Missing import: add `use rmcp::tool;`

### "cannot find attribute `prompt`"
Missing imports: add `use rmcp::prompt;`, `use rmcp::prompt_router;`, and `use rmcp::prompt_handler;`

### Prompts not showing in Claude Desktop
1. Check server capabilities include `.enable_prompts()`
2. Verify prompt_router is added to the struct and initialized
3. Check `#[prompt_handler(router = self.prompt_router)]` is on ServerHandler impl
4. Restart Claude Desktop after config changes

### "module `mcp` is private"
Module not enabled in lib.rs. Add `pub mod mcp;`

### Server starts but Claude can't see tools
1. Check Claude Desktop config path is correct
2. Restart Claude Desktop after config changes
3. Check server stderr for initialization messages
4. Verify micropub auth is configured (`micropub whoami`)
