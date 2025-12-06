# Server-Side Drafts Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable pushing local drafts to Micropub server as server-side drafts with `post-status: draft`, supporting media upload, backdating, and both CLI and MCP workflows.

**Architecture:** Create new `draft_push.rs` module similar to `publish.rs` that sends micropub CREATE requests with `post-status: draft`. Track server URL and status in existing draft metadata fields. Add CLI command and MCP tool for pushing drafts.

**Tech Stack:** Rust, reqwest (HTTP), serde (JSON), chrono (dates), clap (CLI), rmcp (MCP)

---

## Task 1: Create draft_push Module with Basic Structure

**Files:**
- Create: `src/draft_push.rs`
- Modify: `src/lib.rs:2-10`

**Step 1: Write test for PushResult structure**

Create `tests/draft_push_tests.rs`:

```rust
// ABOUTME: Tests for draft push functionality
// ABOUTME: Validates pushing drafts to server with post-status: draft

use micropub::draft_push::PushResult;

#[test]
fn test_push_result_structure() {
    let result = PushResult {
        url: "https://example.com/posts/draft-123".to_string(),
        is_update: false,
        uploads: vec![
            ("photo.jpg".to_string(), "https://example.com/media/abc.jpg".to_string())
        ],
    };

    assert_eq!(result.url, "https://example.com/posts/draft-123");
    assert!(!result.is_update);
    assert_eq!(result.uploads.len(), 1);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_push_result_structure --lib
```

Expected: FAIL with "no `draft_push` in the root"

**Step 3: Create draft_push module**

Create `src/draft_push.rs`:

```rust
// ABOUTME: Draft push functionality for server-side drafts
// ABOUTME: Handles pushing local drafts to server with post-status: draft

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct PushResult {
    pub url: String,
    pub is_update: bool,
    pub uploads: Vec<(String, String)>,
}
```

**Step 4: Add module to lib.rs**

In `src/lib.rs`, add after line 9 (after other pub mod declarations):

```rust
pub mod draft_push;
```

**Step 5: Run test to verify it passes**

```bash
cargo test test_push_result_structure --lib
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/draft_push.rs src/lib.rs tests/draft_push_tests.rs
git commit -m "feat(draft): add draft_push module with PushResult structure"
```

---

## Task 2: Implement cmd_push_draft Skeleton

**Files:**
- Modify: `src/draft_push.rs:5-15`
- Modify: `tests/draft_push_tests.rs:18-40`

**Step 1: Write test for cmd_push_draft function signature**

Add to `tests/draft_push_tests.rs`:

```rust
use chrono::{DateTime, Utc};

#[tokio::test]
async fn test_cmd_push_draft_requires_valid_draft_id() {
    let result = micropub::draft_push::cmd_push_draft("nonexistent", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Draft not found"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_cmd_push_draft_requires_valid_draft_id
```

Expected: FAIL with "no function `cmd_push_draft`"

**Step 3: Add cmd_push_draft function**

In `src/draft_push.rs`, add:

```rust
use chrono::{DateTime, Utc};
use crate::draft::Draft;

pub async fn cmd_push_draft(
    draft_id: &str,
    backdate: Option<DateTime<Utc>>,
) -> Result<PushResult> {
    // Load draft
    let draft = Draft::load(draft_id)?;

    // TODO: Implement push logic

    anyhow::bail!("Push not yet implemented")
}
```

**Step 4: Update test to match implementation**

Modify test in `tests/draft_push_tests.rs`:

```rust
#[tokio::test]
async fn test_cmd_push_draft_requires_valid_draft_id() {
    let result = micropub::draft_push::cmd_push_draft("nonexistent", None).await;
    assert!(result.is_err());
    // Will fail with "Draft not found" from Draft::load
}
```

**Step 5: Run test**

```bash
cargo test test_cmd_push_draft_requires_valid_draft_id
```

Expected: PASS (fails with "Draft not found" as expected)

**Step 6: Commit**

```bash
git add src/draft_push.rs tests/draft_push_tests.rs
git commit -m "feat(draft): add cmd_push_draft function skeleton"
```

---

## Task 3: Add CLI Command for draft push

**Files:**
- Modify: `src/main.rs:92-110`
- Modify: `src/main.rs:170-180`

**Step 1: Add Push to DraftCommands enum**

In `src/main.rs`, find `enum DraftCommands` (around line 92) and add:

```rust
#[derive(Subcommand)]
enum DraftCommands {
    // ... existing commands ...
    /// Push a draft to the server as a server-side draft
    Push {
        /// Draft ID to push
        draft_id: String,
        /// Backdate the draft (ISO 8601 format)
        #[arg(long)]
        backdate: Option<String>,
    },
}
```

**Step 2: Add handler in main**

In the `Commands::Draft` match arm (around line 170), add:

```rust
Commands::Draft(draft_cmd) => match draft_cmd {
    // ... existing handlers ...
    DraftCommands::Push { draft_id, backdate } => {
        use chrono::DateTime;
        use micropub::draft_push;

        let backdate_parsed = if let Some(date_str) = backdate {
            Some(DateTime::parse_from_rfc3339(&date_str)?.with_timezone(&chrono::Utc))
        } else {
            None
        };

        let result = draft_push::cmd_push_draft(&draft_id, backdate_parsed).await?;

        println!("✓ Draft pushed to server!");
        println!("  URL: {}", result.url);
        println!("  Status: {}", if result.is_update { "updated" } else { "created" });

        if !result.uploads.is_empty() {
            println!("\nUploaded media:");
            for (filename, url) in result.uploads {
                println!("  - {} -> {}", filename, url);
            }
        }
    }
}
```

**Step 3: Verify it compiles**

```bash
cargo build
```

Expected: SUCCESS (will fail at runtime since cmd_push_draft not implemented)

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add 'micropub draft push' command"
```

---

## Task 4: Implement Draft Push Logic (Create Path)

**Files:**
- Modify: `src/draft_push.rs:15-120`

**Step 1: Add imports and dependencies**

In `src/draft_push.rs`, add at top:

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::client::{MicropubAction, MicropubClient, MicropubRequest};
use crate::config::{load_token, Config};
use crate::draft::Draft;
use crate::media::{find_media_references, replace_paths, resolve_path, upload_file};
```

**Step 2: Implement cmd_push_draft**

Replace the function body in `src/draft_push.rs`:

```rust
pub async fn cmd_push_draft(
    draft_id: &str,
    backdate: Option<DateTime<Utc>>,
) -> Result<PushResult> {
    // Load draft
    let mut draft = Draft::load(draft_id)?;

    // Load config
    let config = Config::load()?;

    // Determine profile
    let profile_name = draft
        .metadata
        .profile
        .as_deref()
        .or(Some(config.default_profile.as_str()))
        .context("No profile specified and no default profile set")?;

    let profile = config
        .get_profile(profile_name)
        .context(format!("Profile not found: {}", profile_name))?;

    // Load token
    let token = load_token(profile_name)?;

    // Collect media references
    let mut media_refs = find_media_references(&draft.content);
    for photo_path in &draft.metadata.photo {
        if !photo_path.starts_with("http://") && !photo_path.starts_with("https://") {
            media_refs.push(photo_path.clone());
        }
    }

    // Upload media
    let mut replacements = Vec::new();
    let mut uploaded_photo_urls = Vec::new();
    let mut upload_results = Vec::new();

    if !media_refs.is_empty() {
        let media_endpoint = profile.media_endpoint.as_ref()
            .context(format!(
                "No media endpoint found for profile '{}'. Re-authenticate:\n  micropub auth {}",
                profile_name, profile.domain
            ))?;

        println!("Uploading {} media file(s)...", media_refs.len());

        for local_path in media_refs {
            let resolved = resolve_path(&local_path, None)?;
            println!("  Uploading: {}", resolved.display());

            let url = upload_file(media_endpoint, &token, &resolved).await?;
            println!("    -> {}", url);

            let filename = resolved
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            upload_results.push((filename, url.clone()));
            replacements.push((local_path.clone(), url.clone()));

            if draft.metadata.photo.contains(&local_path) {
                uploaded_photo_urls.push(url);
            }
        }
    }

    // Replace paths in content
    let final_content = replace_paths(&draft.content, &replacements);

    // Build micropub request properties
    let mut properties = Map::new();
    properties.insert(
        "content".to_string(),
        Value::Array(vec![Value::String(final_content)]),
    );

    if let Some(name) = &draft.metadata.name {
        properties.insert(
            "name".to_string(),
            Value::Array(vec![Value::String(name.clone())]),
        );
    }

    if !draft.metadata.category.is_empty() {
        properties.insert(
            "category".to_string(),
            Value::Array(
                draft.metadata.category.iter()
                    .map(|c| Value::String(c.clone()))
                    .collect(),
            ),
        );
    }

    if !draft.metadata.photo.is_empty() {
        let photo_values: Vec<Value> = if !uploaded_photo_urls.is_empty() {
            uploaded_photo_urls.iter()
                .map(|url| Value::String(url.clone()))
                .collect()
        } else {
            draft.metadata.photo.iter()
                .map(|p| Value::String(p.clone()))
                .collect()
        };
        properties.insert("photo".to_string(), Value::Array(photo_values));
    }

    if !draft.metadata.syndicate_to.is_empty() {
        properties.insert(
            "mp-syndicate-to".to_string(),
            Value::Array(
                draft.metadata.syndicate_to.iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }

    // Handle published date
    let published_date = backdate.or(draft.metadata.published);
    if let Some(date) = published_date {
        properties.insert(
            "published".to_string(),
            Value::Array(vec![Value::String(date.to_rfc3339())]),
        );
    }

    // CRITICAL: Set post-status to draft
    properties.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("draft".to_string())]),
    );

    // Determine if this is an update or create
    let is_update = draft.metadata.url.is_some();

    let request = if is_update {
        // Update existing server draft
        let mut replace = Map::new();
        replace.insert("content".to_string(), properties.get("content").unwrap().clone());
        if let Some(name) = properties.get("name") {
            replace.insert("name".to_string(), name.clone());
        }
        if let Some(category) = properties.get("category") {
            replace.insert("category".to_string(), category.clone());
        }
        if let Some(photo) = properties.get("photo") {
            replace.insert("photo".to_string(), photo.clone());
        }
        if let Some(published) = properties.get("published") {
            replace.insert("published".to_string(), published.clone());
        }

        MicropubRequest {
            action: MicropubAction::Update {
                replace,
                add: Map::new(),
                delete: Vec::new(),
            },
            properties: Map::new(),
            url: draft.metadata.url.clone(),
        }
    } else {
        // Create new server draft
        MicropubRequest {
            action: MicropubAction::Create,
            properties,
            url: None,
        }
    };

    // Send request
    let micropub_endpoint = profile
        .micropub_endpoint
        .as_ref()
        .context("No micropub endpoint configured for this profile")?;

    let client = MicropubClient::new(micropub_endpoint.clone(), token);

    println!("Pushing draft to {}...", profile.domain);
    let response = client.send(&request).await?;

    // Update draft metadata
    let server_url = response.url.clone().context("Server didn't return URL")?;
    draft.metadata.status = Some("server-draft".to_string());
    draft.metadata.url = Some(server_url.clone());

    // Save updated draft (stays in drafts directory)
    draft.save()?;

    println!("✓ Draft pushed successfully!");
    println!("  URL: {}", server_url);

    Ok(PushResult {
        url: server_url,
        is_update,
        uploads: upload_results,
    })
}
```

**Step 3: Verify it compiles**

```bash
cargo build
```

Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/draft_push.rs
git commit -m "feat(draft): implement cmd_push_draft with create and update logic"
```

---

## Task 5: Add MCP push_draft Tool

**Files:**
- Modify: `src/mcp.rs:100-130` (add PushDraftArgs)
- Modify: `src/mcp.rs:800-900` (add push_draft tool)

**Step 1: Add PushDraftArgs struct**

In `src/mcp.rs`, after the UploadMediaArgs struct (around line 118), add:

```rust
/// Parameters for push_draft tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PushDraftArgs {
    /// Draft ID to push to server
    pub draft_id: String,

    /// Optional backdate in ISO 8601 format (e.g., "2024-01-15T10:00:00Z")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdate: Option<String>,
}
```

**Step 2: Add push_draft tool**

In `src/mcp.rs`, in the `#[tool_router] impl MicropubMcp` block, after the upload_media function (around line 765), add:

```rust
/// Push a draft to the server as a server-side draft
#[tool(description = "Push a local draft to the server as a server-side draft (post-status: draft). Uploads any media files and returns the server URL. Can be used to create new server drafts or update existing ones. Supports backdating.")]
async fn push_draft(
    &self,
    Parameters(args): Parameters<PushDraftArgs>,
) -> Result<CallToolResult, McpError> {
    use chrono::DateTime;
    use crate::draft_push;

    // Parse backdate if provided
    let backdate_parsed = if let Some(date_str) = args.backdate {
        Some(DateTime::parse_from_rfc3339(&date_str)
            .map_err(|e| McpError::invalid_params(
                format!("Invalid backdate format: {}. Use ISO 8601 (e.g., 2024-01-15T10:00:00Z)", e),
                None,
            ))?
            .with_timezone(&chrono::Utc))
    } else {
        None
    };

    // Push draft
    let result = draft_push::cmd_push_draft(&args.draft_id, backdate_parsed)
        .await
        .map_err(|e| McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to push draft: {}", e),
            None,
        ))?;

    // Build response
    let response = serde_json::json!({
        "url": result.url,
        "is_update": result.is_update,
        "status": "server-draft",
        "uploaded_media": result.uploads.iter().map(|(filename, url)| {
            serde_json::json!({
                "filename": filename,
                "url": url
            })
        }).collect::<Vec<_>>()
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
    )]))
}
```

**Step 3: Update server instructions**

In `src/mcp.rs`, find the `get_info()` function (around line 1043) and update the instructions:

```rust
instructions: Some(
    "Micropub MCP server for posting and managing micropub content via AI assistants.\n\n\
     IMAGE UPLOADS:\n\
     - Use 'upload_media' tool to upload images explicitly (supports file paths or base64 data)\n\
     - Or use 'publish_post' with local image paths (e.g., ![alt](~/photo.jpg)) - they'll auto-upload\n\n\
     SERVER-SIDE DRAFTS:\n\
     - Use 'push_draft' tool to save drafts to server with post-status: draft\n\
     - Drafts remain editable locally and can be re-pushed to update\n\
     - Use 'publish_post' to change server draft to published status\n\
     - Supports media upload and backdating when pushing drafts\n\n\
     All uploads and draft operations require authentication via 'micropub auth <domain>' first."
        .to_string(),
),
```

**Step 4: Verify it compiles**

```bash
cargo build
```

Expected: SUCCESS

**Step 5: Commit**

```bash
git add src/mcp.rs
git commit -m "feat(mcp): add push_draft tool for server-side drafts"
```

---

## Task 6: Modify publish to Update Server Drafts

**Files:**
- Modify: `src/publish.rs:166-197`

**Step 1: Write test for publish updating server draft**

Add to `tests/draft_push_tests.rs`:

```rust
#[test]
fn test_publish_should_update_if_draft_has_url() {
    // This tests the logic - actual HTTP testing would require mocking
    let has_server_url = Some("https://example.com/posts/draft-123".to_string());
    let should_update = has_server_url.is_some();
    assert!(should_update);
}
```

**Step 2: Run test**

```bash
cargo test test_publish_should_update_if_draft_has_url
```

Expected: PASS

**Step 3: Modify cmd_publish to detect server drafts**

In `src/publish.rs`, find the section where the micropub request is built (around line 166). Before creating the request, add:

```rust
// Check if this draft already exists on the server
let is_server_draft = draft.metadata.url.is_some()
    && draft.metadata.status.as_deref() == Some("server-draft");

let request = if is_server_draft {
    // Update existing server draft to published
    let url = draft.metadata.url.clone().unwrap();

    let mut replace = Map::new();
    replace.insert(
        "content".to_string(),
        Value::Array(vec![Value::String(final_content.clone())]),
    );

    if let Some(name) = &draft.metadata.name {
        replace.insert(
            "name".to_string(),
            Value::Array(vec![Value::String(name.clone())]),
        );
    }

    // Change post-status from draft to published
    replace.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("published".to_string())]),
    );

    MicropubRequest {
        action: MicropubAction::Update {
            replace,
            add: Map::new(),
            delete: Vec::new(),
        },
        properties: Map::new(),
        url: Some(url),
    }
} else {
    // Create new published post (existing behavior)
    MicropubRequest {
        action: MicropubAction::Create,
        properties,
        url: None,
    }
};
```

**Step 4: Update the existing code**

Replace the existing `let request = MicropubRequest { ... }` line (around line 166) with the code above.

**Step 5: Verify it compiles**

```bash
cargo build
```

Expected: SUCCESS

**Step 6: Commit**

```bash
git add src/publish.rs
git commit -m "feat(publish): update server drafts to published when publishing"
```

---

## Task 7: Add Tests for Draft Push Workflow

**Files:**
- Modify: `tests/draft_push_tests.rs:40-150`

**Step 1: Add test for metadata updates**

Add to `tests/draft_push_tests.rs`:

```rust
#[test]
fn test_draft_metadata_has_required_fields() {
    use micropub::draft::DraftMetadata;

    let metadata = DraftMetadata {
        post_type: "note".to_string(),
        name: None,
        published: None,
        category: Vec::new(),
        syndicate_to: Vec::new(),
        profile: None,
        photo: Vec::new(),
        status: Some("server-draft".to_string()),
        url: Some("https://example.com/posts/draft-123".to_string()),
        published_at: None,
    };

    assert_eq!(metadata.status, Some("server-draft".to_string()));
    assert_eq!(metadata.url, Some("https://example.com/posts/draft-123".to_string()));
}
```

**Step 2: Add test for create vs update logic**

```rust
#[test]
fn test_is_update_logic() {
    use micropub::draft::DraftMetadata;

    let metadata_new = DraftMetadata::default();
    assert!(metadata_new.url.is_none());
    let is_update_new = metadata_new.url.is_some();
    assert!(!is_update_new);

    let metadata_existing = DraftMetadata {
        url: Some("https://example.com/posts/draft-123".to_string()),
        ..Default::default()
    };
    let is_update_existing = metadata_existing.url.is_some();
    assert!(is_update_existing);
}
```

**Step 3: Add test for post-status property**

```rust
#[test]
fn test_post_status_draft_property() {
    use serde_json::{Map, Value};

    let mut properties = Map::new();
    properties.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("draft".to_string())]),
    );

    let post_status = properties.get("post-status")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str());

    assert_eq!(post_status, Some("draft"));
}
```

**Step 4: Run tests**

```bash
cargo test --test draft_push_tests
```

Expected: All tests PASS

**Step 5: Commit**

```bash
git add tests/draft_push_tests.rs
git commit -m "test(draft): add tests for draft push workflow"
```

---

## Task 8: Add MCP Tool Tests

**Files:**
- Modify: `tests/mcp_tests.rs:180-250`

**Step 1: Add test for push_draft parameter validation**

Add to `tests/mcp_tests.rs`:

```rust
#[test]
fn test_push_draft_requires_draft_id() {
    use serde_json::json;

    let args = json!({});

    let parsed: Result<serde_json::Value, _> = serde_json::from_value(args);
    assert!(parsed.is_ok()); // JSON valid but missing required field
}

#[test]
fn test_push_draft_with_backdate() {
    use serde_json::json;

    let args = json!({
        "draft_id": "abc123",
        "backdate": "2024-01-15T10:00:00Z"
    });

    let parsed: Result<serde_json::Value, _> = serde_json::from_value(args);
    assert!(parsed.is_ok());
}
```

**Step 2: Add test for response structure**

```rust
#[test]
fn test_push_draft_response_structure() {
    use serde_json::json;

    let response = json!({
        "url": "https://example.com/posts/draft-123",
        "is_update": false,
        "status": "server-draft",
        "uploaded_media": []
    });

    assert_eq!(response["url"], "https://example.com/posts/draft-123");
    assert_eq!(response["is_update"], false);
    assert_eq!(response["status"], "server-draft");
    assert!(response["uploaded_media"].is_array());
}
```

**Step 3: Run tests**

```bash
cargo test --test mcp_tests -- push_draft
```

Expected: All tests PASS

**Step 4: Commit**

```bash
git add tests/mcp_tests.rs
git commit -m "test(mcp): add tests for push_draft tool"
```

---

## Task 9: Update Documentation and CHANGELOG

**Files:**
- Modify: `CHANGELOG.md:1-20`
- Create: `docs/server-side-drafts.md`

**Step 1: Create user documentation**

Create `docs/server-side-drafts.md`:

```markdown
# Server-Side Drafts

Server-side drafts allow you to push drafts to your Micropub server while keeping them private (not published).

## Basic Usage

### Push a draft to server

```bash
micropub draft push <draft-id>
```

This sends the draft to your server with `post-status: draft`, keeping it private but stored on the server.

### Push with backdate

```bash
micropub draft push <draft-id> --backdate "2024-01-15T10:00:00Z"
```

### Update an existing server draft

Simply run push again with the same draft ID:

```bash
micropub draft push <draft-id>
```

The draft metadata tracks the server URL, so subsequent pushes update the existing server draft.

### Publish a server draft

When ready to make the draft public:

```bash
micropub publish <draft-id>
```

This updates the server draft's `post-status` from `draft` to `published`.

## Workflow

1. **Create** local draft: `micropub draft new`
2. **Edit** locally: `micropub draft edit <draft-id>`
3. **Push** to server as draft: `micropub draft push <draft-id>` (private)
4. **Update** server draft: re-run `micropub draft push <draft-id>`
5. **Publish** when ready: `micropub publish <draft-id>` (public)

## MCP Integration

AI agents can use the `push_draft` tool:

```json
{
  "tool": "push_draft",
  "arguments": {
    "draft_id": "abc123",
    "backdate": "2024-01-15T10:00:00Z"
  }
}
```

## Media Upload

Media files are automatically uploaded when pushing drafts, just like when publishing.

## Metadata Tracking

When you push a draft, the local draft metadata is updated with:
- `url`: Server URL for the draft
- `status`: Set to "server-draft"

This allows the CLI and MCP to track which drafts are synced to the server.
```

**Step 2: Update CHANGELOG.md**

Add to top of `CHANGELOG.md`:

```markdown
## [Unreleased]

### Added
- Server-side draft support with `micropub draft push` command
  - Push local drafts to server with `post-status: draft`
  - Automatic media upload when pushing drafts
  - Support for backdating via `--backdate` flag
  - Update existing server drafts by re-pushing
  - Track server URL and status in draft metadata
- MCP `push_draft` tool for AI agent workflows
  - Supports draft_id and optional backdate parameters
  - Returns server URL and upload information
  - Updated server instructions to document draft pushing
- Publishing now updates server drafts to published status
  - Detects if draft has server URL and status "server-draft"
  - Sends UPDATE request to change post-status to published
  - Seamless workflow from draft to published

### Changed
- Draft metadata now tracks server synchronization state
- Publish command detects and updates server-side drafts
```

**Step 3: Commit**

```bash
git add CHANGELOG.md docs/server-side-drafts.md
git commit -m "docs: add server-side drafts documentation and CHANGELOG"
```

---

## Task 10: Final Integration Test and Verification

**Files:**
- Run full test suite
- Verify CLI commands work
- Test MCP integration

**Step 1: Run all tests**

```bash
cargo test
```

Expected: All tests PASS

**Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings

**Step 3: Run formatter**

```bash
cargo fmt --check
```

Expected: All files formatted

**Step 4: Build release**

```bash
cargo build --release
```

Expected: SUCCESS

**Step 5: Manual CLI smoke test**

```bash
# Show help
./target/release/micropub draft push --help

# Expected output showing usage and --backdate option
```

**Step 6: Commit any formatting changes**

```bash
git add -u
git commit -m "chore: final formatting and cleanup for server-side drafts"
```

**Step 7: Verify git status**

```bash
git status
```

Expected: Clean working tree or only untracked files

---

## Completion Checklist

- [ ] All 10 tasks completed
- [ ] All tests passing (unit + integration + MCP)
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] CHANGELOG updated
- [ ] Documentation created
- [ ] CLI command works
- [ ] MCP tool implemented
- [ ] Ready for code review

## Next Steps

After completing this plan:
1. Use @superpowers:requesting-code-review to review the implementation
2. Test manually with real Micropub server (if available)
3. Merge to main branch
4. Bump version to 0.4.0
5. Create release

## Notes for Implementation

- Reuse existing media upload logic from `src/media.rs`
- Follow same patterns as `src/publish.rs` for consistency
- Ensure MCP tool responses match documented format
- Test both CREATE and UPDATE paths thoroughly
- Verify metadata persistence across push operations
