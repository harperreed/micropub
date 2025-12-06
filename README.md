# Micropub CLI

An ultra-compliant Micropub CLI for interacting with Micropub-enabled sites like micro.blog.

## Features

- ✅ Full W3C Micropub spec compliance
- ✅ CLI-managed drafts with YAML frontmatter
- ✅ Server-side draft support (push drafts to server before publishing)
- ✅ Automatic media upload and URL replacement
- ✅ Multi-site support with profiles
- ✅ IndieAuth/OAuth authentication
- ✅ Create, update, delete, undelete posts
- ✅ Backdated post publishing
- ✅ XDG-compliant configuration storage

## Installation

### Homebrew (macOS)

```bash
brew tap harperreed/tap
brew install micropub
```

### From Source

```bash
cargo install --path .
```

## Quick Start

1. **Authenticate with your site:**
   ```bash
   micropub auth micro.blog
   ```

2. **Create a new draft:**
   ```bash
   micropub draft new
   ```

3. **List drafts:**
   ```bash
   micropub draft list
   ```

4. **Push draft to server (optional):**
   ```bash
   micropub draft push <draft-id>
   # Or backdate when pushing
   micropub draft push <draft-id> --backdate "2023-12-25"
   ```

5. **Publish a draft:**
   ```bash
   micropub publish <draft-id>
   ```

See [USAGE.md](docs/USAGE.md) for detailed documentation.

## Server-side Draft Workflow

Micropub supports pushing drafts to your server before publishing, allowing you to preview and edit posts on your site:

1. **Create and edit a local draft:**
   ```bash
   micropub draft new
   # Edit the draft file in your editor
   ```

2. **Push draft to server (with draft status):**
   ```bash
   micropub draft push my-draft-id
   ```
   The draft is now on your server but not published (marked as `post-status: draft`)

3. **Update the draft:**
   ```bash
   # Edit the local draft file
   micropub draft push my-draft-id  # Re-push to update server version
   ```

4. **Publish when ready:**
   ```bash
   micropub publish my-draft-id
   ```
   This changes the server post from draft to published status

**Benefits:**
- Preview drafts on your site before publishing
- Edit and iterate on server-side drafts
- Works with backdating: `micropub draft push <id> --backdate "2023-12-25"`
- Available via MCP for AI assistant workflows

## Architecture

- `config` - Configuration and XDG directory management
- `auth` - IndieAuth/OAuth authentication
- `draft` - Draft lifecycle management
- `client` - Micropub HTTP client
- `media` - Media upload and path replacement
- `publish` - Post publishing orchestration
- `operations` - Update/delete/undelete operations

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Development

Built with Rust using:
- clap - CLI argument parsing
- reqwest - HTTP client
- tokio - Async runtime
- serde - Serialization
- oauth2 - OAuth flow

## License

MIT
