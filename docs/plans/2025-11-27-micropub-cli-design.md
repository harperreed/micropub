# Micropub CLI Design Document

**Date:** 2025-11-27
**Status:** Draft
**Goal:** Ultra-compliant Micropub CLI for personal blogging with power user features

## Overview

A Rust-based command-line interface for interacting with Micropub-enabled sites (like micro.blog). The tool prioritizes strict W3C spec compliance while providing power user features including CLI-managed drafts, automatic media handling, multi-site support, and comprehensive post management.

## Core Requirements

- **Full W3C Micropub spec compliance** - Support all required and optional features, proper error handling, correct content negotiation
- **CLI-managed draft workflow** - Create, edit, list, and publish drafts with the CLI managing storage
- **Automatic media handling** - Parse draft content for local file references, auto-upload, replace with URLs
- **Multi-site support** - Named profiles with default selection and per-draft overrides
- **Post management** - Create, update, delete, undelete operations
- **Backdated posts** - Explicit separate command for publishing with historical timestamps
- **Secure authentication** - Interactive IndieAuth/OAuth flow with XDG-compliant token storage

## Architecture

### Module Structure

**Auth Module**
- IndieAuth endpoint discovery (parse HTML/headers for rel links)
- OAuth2 authorization code flow
- Token storage in `~/.local/share/micropub/tokens/<profile>.token` (0600 permissions)
- Token validation and refresh

**Draft Module**
- Draft lifecycle management (create, edit, list, show)
- YAML frontmatter + Markdown content parsing
- Draft storage in `~/.local/share/micropub/drafts/<uuid>.md`
- Archive published drafts to `~/.local/share/micropub/archive/<uuid>.md`
- Add publication metadata to archived drafts (status, URL, timestamp)

**Client Module**
- HTTP client for Micropub endpoint interactions
- Endpoint discovery (rel="micropub", rel="media-endpoint")
- All Micropub operations: create, update, delete, undelete
- Query endpoints: config, source, syndicate-to
- Support both JSON and form-encoded formats
- Proper error response parsing

**Media Module**
- Scan draft content for local file references
- Support multiple formats: markdown images, HTML img tags, frontmatter photo arrays
- Path resolution (expand `~`, handle relative paths)
- MIME type detection
- Upload to media endpoint with multipart/form-data
- URL replacement in content

**Config Module**
- Manage site profiles in `~/.config/micropub/config.toml`
- User preferences (default profile, editor)
- XDG directory resolution

### CLI Command Structure

```
micropub auth <domain>              # Start OAuth flow for a site
micropub draft new                  # Create new draft, open in $EDITOR
micropub draft edit <draft-id>      # Edit existing draft
micropub draft list                 # Show all drafts
micropub draft show <draft-id>      # Display draft content
micropub publish <draft>            # Publish a draft (archives it)
micropub backdate <draft> --date <datetime>  # Publish with backdated timestamp
micropub update <url>               # Update existing post
micropub delete <url>               # Delete post
micropub undelete <url>             # Undelete post
micropub debug <profile>            # Test connectivity, token validity
```

## Draft Format

Markdown files with YAML frontmatter:

```markdown
---
type: article
name: "My Blog Post Title"
published: 2024-01-15T10:30:00Z  # optional, defaults to now
category:
  - rust
  - micropub
syndicate-to:
  - https://twitter.com/username
profile: micro.blog  # optional, uses default if not specified
---

This is the content of my post.

I can reference local images: ![alt text](~/Pictures/photo.jpg)

The CLI will upload that image and replace the path.
```

### Draft Workflow

1. `micropub draft new` - Creates new draft with UUID, opens in `$EDITOR` (from config or env)
2. User edits markdown file with their preferred editor
3. `micropub draft list` - Shows all drafts with titles, dates, IDs
4. `micropub publish drafts/<uuid>.md`:
   - Parse YAML frontmatter for metadata
   - Scan content for local file paths
   - Upload media files via media endpoint
   - Replace local paths with uploaded URLs
   - Send micropub request with all properties
   - Archive draft to `~/.local/share/micropub/archive/<uuid>.md`
   - Add metadata: `status: published`, `url: <response-url>`, `published_at: <timestamp>`

## Configuration

### Config File Format

`~/.config/micropub/config.toml`:

```toml
# Default profile to use when not specified
default_profile = "micro.blog"

# Editor for drafts (falls back to $EDITOR env var)
editor = "vim"

[profiles.micro.blog]
domain = "micro.blog"
micropub_endpoint = "https://micro.blog/micropub"
media_endpoint = "https://micro.blog/micropub/media"
# Token stored separately in ~/.local/share/micropub/tokens/micro.blog.token

[profiles.personal]
domain = "myblog.example.com"
# Endpoints discovered via IndieAuth/micropub discovery
```

### Multi-Site Flow

1. `micropub auth micro.blog` - Performs OAuth, discovers endpoints, saves profile and token
2. Draft specifies `profile: personal` in frontmatter to override default
3. Commands accept `--profile personal` flag to override
4. Tokens stored separately for security: `~/.local/share/micropub/tokens/<profile-name>.token` (mode 0600)

### Discovery Process

When authenticating:
1. Fetch `https://<domain>/` and parse for `rel="micropub"` and `rel="authorization_endpoint"`
2. Start OAuth flow with discovered authorization endpoint
3. Exchange authorization code for access token
4. Query micropub endpoint for capabilities (GET request)
5. Discover media endpoint, supported features, syndication targets
6. Store all discovered endpoints in profile config

## Media Handling

### Supported Reference Formats

- Markdown images: `![alt](~/path/to/image.jpg)`, `![alt](/absolute/path.jpg)`, `![alt](relative/path.jpg)`
- Markdown links to media: `[download](~/files/document.pdf)`
- HTML img tags: `<img src="~/path/image.png">`
- Frontmatter photo array: `photo: ["~/pics/1.jpg", "~/pics/2.jpg"]`

### Upload Process

1. Parse draft content and frontmatter for file paths
2. Resolve paths (expand `~`, handle relative paths from draft directory)
3. Detect MIME type from file extension and content
4. Upload each file to media endpoint with `multipart/form-data`
5. Collect returned URLs from media endpoint responses
6. Replace all local paths in content with uploaded URLs
7. Send final micropub request with updated content

### Error Handling

- If media upload fails, abort publish and show detailed error
- Validate file exists before attempting upload
- Check file size (configurable, default 10MB warning)
- Support `--skip-media` flag to disable auto-upload if needed

## Spec Compliance

### W3C Micropub Specification Coverage

**Discovery & Negotiation:**
- Follow rel-based endpoint discovery from HTML and HTTP Link headers
- Support `rel="micropub"`, `rel="media-endpoint"`, `rel="authorization_endpoint"`
- Send proper `Authorization: Bearer <token>` headers
- Accept both `application/json` and `application/x-www-form-urlencoded` responses

**Required Operations:**
- ✅ Create posts (both form-encoded and JSON syntax)
- ✅ Update posts (replace, add, delete properties)
- ✅ Delete posts (`action=delete`)
- ✅ Undelete posts (`action=undelete`)
- ✅ Media endpoint support
- ✅ Configuration queries (`q=config`, `q=source`, `q=syndicate-to`)

### Error Handling

**Micropub Error Response Parsing:**
- Parse `error` and `error_description` fields from responses
- Map standard error codes to helpful messages:
  - `insufficient_scope` → "Your token doesn't have permission. Re-authenticate with `micropub auth <domain>`"
  - `invalid_request` → Show server's error_description, validate draft format
  - `unauthorized` → "Token invalid or expired. Re-authenticate."
  - Network errors → Suggest checking connection, endpoint availability

**Validation:**
- Validate draft format before sending (required fields present, valid YAML)
- Check for common mistakes (missing content, invalid dates, malformed URLs)
- Dry-run mode: `micropub publish --dry-run <draft>` shows what would be sent without posting

**Logging & Debugging:**
- `--verbose` flag shows full HTTP requests/responses
- Logs stored in `~/.local/share/micropub/logs/micropub.log`
- `micropub debug <profile>` command tests endpoint connectivity, token validity, capabilities

## Implementation Details

### Rust Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.11", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
url = "2"
dirs = "5"
mime_guess = "2"
oauth2 = "4"
scraper = "0.18"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

### Core Data Structures

```rust
// Draft representation
struct Draft {
    id: String,  // UUID
    metadata: DraftMetadata,
    content: String,
    file_path: PathBuf,
}

struct DraftMetadata {
    post_type: PostType,  // note, article, photo, etc.
    name: Option<String>,  // title for articles
    published: Option<DateTime<Utc>>,
    category: Vec<String>,
    syndicate_to: Vec<String>,
    profile: Option<String>,
    photo: Vec<String>,  // Can be URLs or local paths
    // ... other micropub properties
}

enum PostType {
    Note,
    Article,
    Photo,
    Reply,
    Repost,
    Like,
    Bookmark,
    // ... other post types
}

// Micropub request
struct MicropubRequest {
    action: Action,
    properties: HashMap<String, serde_json::Value>,
    url: Option<String>,  // for update/delete/undelete
}

enum Action {
    Create,
    Update { replace: HashMap, add: HashMap, delete: Vec },
    Delete,
    Undelete,
}

// Profile configuration
struct Profile {
    domain: String,
    micropub_endpoint: String,
    media_endpoint: Option<String>,
    token_endpoint: String,
    authorization_endpoint: String,
}
```

### Async Strategy

- Use async/await throughout for HTTP operations
- CLI commands are sync wrappers around async functions
- Use Tokio runtime for async execution

## Testing Strategy

### Unit Tests

- Draft parsing (YAML frontmatter + markdown content separation)
- Media path detection and replacement logic
- Config file parsing and XDG path resolution
- Micropub request serialization (both form-encoded and JSON formats)
- Error response parsing and user-friendly message mapping
- URL discovery from HTML responses

### Integration Tests

- Mock HTTP server for micropub endpoint testing
- Test full OAuth flow with mock authorization server
- Test media upload with mock media endpoint
- Test all CRUD operations (create, update, delete, undelete)
- Test endpoint discovery from HTML with mock responses
- Test draft archival and metadata updates

### End-to-End Tests

- Test against a local micropub test server
- Or use micro.blog's test/sandbox endpoint if available
- Validate actual OAuth flow completion
- Test real media uploads and URL replacement
- Test multi-site profile switching

### Compliance Testing

- Run against micropub.rocks test suite to validate spec compliance
- Document which test cases pass
- Use `--dry-run` mode extensively in tests
- Test error handling for all defined error codes
- Validate request formatting matches spec examples

### Test Structure

```
src/
  lib.rs              # Library code with inline unit tests
  auth.rs             # Auth module with #[cfg(test)] tests
  draft.rs            # Draft module with #[cfg(test)] tests
  client.rs           # Client module with #[cfg(test)] tests
  media.rs            # Media module with #[cfg(test)] tests
  config.rs           # Config module with #[cfg(test)] tests

tests/
  integration_tests.rs   # Integration tests with mock servers
  compliance_tests.rs    # Spec compliance validation

fixtures/
  example_drafts/        # Sample drafts for testing
  mock_responses/        # Mock HTTP responses
```

## Future Enhancements

(Not in scope for v1, but good to consider)

- Search published posts by date, category, content
- Batch operations (publish multiple drafts, bulk updates)
- Template system for common post types
- Syndication status tracking
- Offline draft editing with sync later
- Post preview/validation before publishing
- Shell completion scripts
- Migration tools from other platforms

## Success Criteria

- [ ] Passes all micropub.rocks compliance tests
- [ ] Successfully authenticates with micro.blog and personal sites
- [ ] Can create, update, delete posts on real micropub endpoints
- [ ] Handles media uploads and path replacement correctly
- [ ] Properly stores and manages drafts with archival
- [ ] Multi-site profiles work seamlessly
- [ ] Error messages are helpful and actionable
- [ ] All core commands have comprehensive tests
- [ ] Documentation covers installation, setup, and usage
