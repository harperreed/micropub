# Server-Side Draft Feature Design

**Date:** 2025-12-04
**Status:** Approved
**Version:** 0.4.0 target

## Overview

Enable pushing local drafts to the Micropub server as server-side drafts with `post-status: draft`. Drafts remain editable locally and can be re-pushed to update the server version.

## Goals

- Explicit `micropub draft push <draft-id>` command (no auto-sync)
- Track both server URL and status in local draft metadata
- Support media upload when pushing drafts
- Support backdating via `--backdate` flag
- Update existing server drafts if already pushed
- Full MCP integration for AI agent workflows

## Architecture

### Draft Metadata Tracking

Use existing fields in `DraftMetadata`:
- `url: Option<String>` - server URL for the draft (already exists)
- `status: Option<String>` - set to "server-draft" when pushed

### Push Workflow

Similar to existing publish pattern but sends `post-status: draft`:

1. Load draft and resolve media references
2. Upload media files to media endpoint
3. Send micropub request with `post-status: draft`
4. Store returned URL and update status locally
5. Keep draft in drafts directory (not archived)

### MCP Integration

New `push_draft` tool with:
- `draft_id` and optional `backdate` parameters
- Structured response with server URL and upload details
- Updated MCP server instructions documenting draft pushing

## Command Interface

### CLI Command

```bash
micropub draft push <draft-id> [--backdate <date>]
```

**Examples:**
```bash
# Push draft to server
micropub draft push abc123

# Push with specific publish date
micropub draft push abc123 --backdate "2024-01-15T10:00:00Z"

# Re-push to update existing server draft
micropub draft push abc123  # uses existing URL, sends update
```

### Command Behavior

**First push (no URL in metadata):**
- Uploads media files
- Sends CREATE request with `post-status: draft`
- Stores returned URL in draft metadata
- Sets `status: server-draft`
- Keeps draft in local drafts directory

**Subsequent push (URL exists in metadata):**
- Uploads any new media files
- Sends UPDATE request to existing URL
- Keeps `post-status: draft`
- Updates `status: server-draft` timestamp

**After pushing, publishing the draft:**
- `micropub publish <draft-id>` sends UPDATE to change `post-status: published`
- Archives draft locally as usual
- Sets `status: published`

### MCP Tool

```json
{
  "tool": "push_draft",
  "arguments": {
    "draft_id": "abc123",
    "backdate": "2024-01-15T10:00:00Z"
  }
}
```

**Response:**
```json
{
  "url": "https://server.com/posts/draft-12345",
  "is_update": false,
  "status": "server-draft",
  "uploaded_media": [
    {"filename": "photo.jpg", "url": "https://server.com/media/abc.jpg"}
  ]
}
```

## Implementation Structure

### New Module: `src/draft_push.rs`

```rust
pub async fn cmd_push_draft(
    draft_id: &str,
    backdate: Option<DateTime<Utc>>,
) -> Result<PushResult> {
    // Load draft
    // Upload media (reuse media::upload_file)
    // Build micropub request with post-status: draft
    // Send CREATE or UPDATE based on existing URL
    // Update draft metadata (url, status)
    // Save draft back to drafts directory
    // Return PushResult with URL and upload info
}

pub struct PushResult {
    pub url: String,
    pub is_update: bool,
    pub uploads: Vec<(String, String)>,  // (filename, url)
}
```

### Modified Files

**`src/main.rs`** - Add new subcommand:
```rust
#[derive(Subcommand)]
enum DraftCommands {
    // ... existing commands ...
    Push {
        draft_id: String,
        #[arg(long)]
        backdate: Option<String>,
    },
}
```

**`src/draft.rs`** - No changes needed (already has `url` and `status` fields)

**`src/client.rs`** - Add UPDATE action support:
```rust
pub enum MicropubAction {
    Create,
    Update,  // NEW
    Delete,
    Undelete,
}
```

**`src/mcp.rs`** - Add new tool:
```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PushDraftArgs {
    pub draft_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdate: Option<String>,
}

#[tool(description = "Push a local draft to the server as a server-side draft...")]
async fn push_draft(
    &self,
    Parameters(args): Parameters<PushDraftArgs>,
) -> Result<CallToolResult, McpError>
```

## Micropub Protocol Details

### CREATE Request (first push)

```json
{
  "type": ["h-entry"],
  "properties": {
    "content": ["Draft content with uploaded media URLs"],
    "name": ["Optional title"],
    "category": ["tags", "if", "present"],
    "photo": ["https://server.com/media/uploaded.jpg"],
    "published": ["2024-01-15T10:00:00Z"],
    "post-status": ["draft"]
  }
}
```

**Server Response:**
```
HTTP 201 Created
Location: https://server.com/posts/draft-12345
```

### UPDATE Request (re-push)

```json
{
  "action": "update",
  "url": "https://server.com/posts/draft-12345",
  "replace": {
    "content": ["Updated content"],
    "photo": ["new uploaded media URLs"]
  }
}
```

### Publishing a Server Draft

When running `micropub publish <draft-id>` on a draft with server URL:

```json
{
  "action": "update",
  "url": "https://server.com/posts/draft-12345",
  "replace": {
    "post-status": ["published"]
  }
}
```

## Error Handling

- **No media endpoint**: Clear error message to re-authenticate
- **Server doesn't support drafts**: Graceful fallback or clear error
- **Network failures**: Retry logic or save state for recovery
- **URL conflicts**: Handle if server URL changes between pushes
- **Draft already published**: Warn user, suggest using `micropub update`
- **Server URL becomes invalid**: If UPDATE fails with 404, offer to CREATE new draft
- **Media upload failures**: Fail early before sending draft content
- **Server doesn't support post-status**: Detect and provide clear error

## Testing Strategy

### Unit Tests (tests/draft_push_tests.rs)
- Test draft metadata updates (URL, status)
- Test CREATE vs UPDATE decision logic
- Test backdate parameter handling
- Test media reference collection

### Integration Tests
- Mock micropub server responses
- Test full push workflow with media
- Test re-push updates existing draft
- Test publish after push (draft → published)

### MCP Tool Tests (tests/mcp_tests.rs)
- Test push_draft tool parameter validation
- Test response format includes URL and uploads
- Test error messages are clear

## MCP Integration Details

### Updated Server Instructions

Add to MCP `get_info()` instructions:

```
SERVER-SIDE DRAFTS:
- Use 'push_draft' tool to save drafts to server with post-status: draft
- Drafts remain editable locally and can be re-pushed to update
- Use 'publish_post' to change server draft to published status
- Supports media upload and backdating when pushing drafts
```

### New Prompt: `draft_workflow`

```
I'll help you manage server-side drafts. You can:
1. Create local draft with 'create_draft'
2. Push to server as draft with 'push_draft' (stays editable)
3. Update and re-push with 'push_draft' (updates server version)
4. Publish when ready with 'publish_post' (marks as published)

This workflow lets you work on posts over time while keeping them private.
```

## Implementation Plan

1. **Add UPDATE support to client** (src/client.rs)
   - Add `Update` variant to `MicropubAction`
   - Implement update request serialization

2. **Create draft push module** (src/draft_push.rs)
   - Implement `cmd_push_draft` function
   - Handle CREATE vs UPDATE logic
   - Media upload integration
   - Metadata updates

3. **Add CLI command** (src/main.rs)
   - Add `Push` to `DraftCommands`
   - Wire up to `cmd_push_draft`

4. **Add MCP tool** (src/mcp.rs)
   - Create `PushDraftArgs` struct
   - Implement `push_draft` tool
   - Update server instructions
   - Add draft workflow prompt

5. **Modify publish behavior** (src/publish.rs)
   - Detect if draft has server URL
   - Send UPDATE instead of CREATE if URL exists
   - Change `post-status: draft` to `post-status: published`

6. **Add tests**
   - Unit tests for push logic
   - MCP tool tests
   - Integration tests for full workflow

7. **Update documentation**
   - Add to CHANGELOG
   - Update README with draft push examples
   - Document MCP draft workflow

## Workflow Summary

Clean draft lifecycle:

1. **Create** drafts locally: `micropub draft new`
2. **Edit** drafts locally: `micropub draft edit <id>`
3. **Push** to server as drafts: `micropub draft push <id>` (keeps private)
4. **Update** server drafts: re-run `micropub draft push <id>`
5. **Publish** when ready: `micropub publish <id>` (goes public)

All functionality works in both CLI and MCP, with full media upload and backdating support.

## Edge Cases

1. **Draft already published**: Warn and suggest `micropub update`
2. **Server URL becomes invalid**: Offer to CREATE new draft on 404
3. **Media upload failures**: Fail early, don't update metadata
4. **Conflicting metadata**: `--backdate` overrides draft's published date
5. **Network interruptions**: Transaction-like behavior, update metadata only on success
6. **Partial failures**: Log uploaded URLs if media succeeds but draft fails
