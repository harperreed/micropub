# Micropub CLI

An ultra-compliant Micropub CLI for interacting with Micropub-enabled sites like micro.blog.

## Features

- ✅ Full W3C Micropub spec compliance
- ✅ CLI-managed drafts with YAML frontmatter
- ✅ Automatic media upload and URL replacement
- ✅ Multi-site support with profiles
- ✅ IndieAuth/OAuth authentication
- ✅ Create, update, delete, undelete posts
- ✅ Backdated post publishing
- ✅ XDG-compliant configuration storage

## Installation

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

4. **Publish a draft:**
   ```bash
   micropub publish <draft-id>
   ```

See [USAGE.md](docs/USAGE.md) for detailed documentation.

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
