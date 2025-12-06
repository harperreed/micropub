# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2025-12-06

### Added
- **Server-side Draft Support**: Push local drafts to server with `post-status: draft`
  - New CLI command: `micropub draft push <draft-id> [--backdate <date>]`
  - New MCP tool: `push_draft` for AI assistant integration
  - Drafts can be re-pushed to update server version without creating duplicates
  - Publishing server drafts sends UPDATE request to change status from draft to published
  - Supports media upload and backdating when pushing drafts
  - `draft list` command shows server status (local-only vs. on-server)
- MCP `upload_media` tool for explicit image uploads
  - Supports file path and base64 data upload
  - Returns structured JSON with URL, filename, mime_type, and markdown
  - Optional alt text for accessibility
- Enhanced `publish_post` and `publish_backdate` with upload feedback
  - Shows which files were uploaded and their URLs
  - Auto-detects and uploads local image references
- Updated server instructions for upload feature discoverability
- Enhanced photo-post prompt with upload method guidance

### Changed
- `cmd_publish` now returns upload details (filename, URL pairs)
- Tool descriptions updated to mention automatic image upload
- Draft metadata now tracks server URL and publish status for pushed drafts

## [0.2.0] - 2025-01-28

### Added
- **MCP Server Support**: Complete Model Context Protocol server implementation
  - 10 MCP tools for posting and managing micropub content
  - 6 workflow prompts for common posting patterns
  - Full integration with Claude Desktop and other MCP clients
- **MCP Tools**:
  - `publish_post` - Create and publish posts immediately
  - `create_draft` - Save drafts for later editing
  - `list_drafts` - View all saved drafts
  - `view_draft` - Read specific draft content
  - `publish_backdate` - Publish posts with past timestamps
  - `delete_post` - Remove published posts
  - `list_posts` - View published posts with pagination
  - `list_media` - View uploaded media files
  - `whoami` - Check authentication status
- **MCP Prompts**:
  - `quick-note` - Post a quick note or thought
  - `photo-post` - Create a photo post with caption
  - `article-draft` - Create longer article drafts
  - `backdate-memory` - Record memories with original dates
  - `categorized-post` - Create posts with specific categories
  - `new-post` - General posting workflow guide

### Security
- Comprehensive input validation on all MCP parameters
- Path traversal protection for draft IDs
- Empty input validation with proper error messages
- Length limits on all text inputs via JSON Schema
- Runtime validation in addition to schema validation
- Whitespace normalization to prevent formatting issues

### Fixed
- Integration tests no longer pollute production config/data
- Proper error handling throughout MCP implementation
- Clean, professional prompt messages without extra whitespace

### Documentation
- Complete MCP integration guide in `MCP.md`
- Usage examples for all tools and prompts
- Configuration instructions for Claude Desktop
- Troubleshooting section for common issues

## [0.1.1] - 2025-01-13

### Added
- Initial public release
- Complete TUI implementation with all features
- Posts and Media list display with dates
- Crates.io metadata

### Changed
- Consolidated CI workflows into single efficient pipeline
- Updated license to MIT only
- Updated author email

### Fixed
- Extract URL path segment for posts without titles
- Improved Posts and Media list display formatting

[0.2.0]: https://github.com/harperreed/micropub/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/harperreed/micropub/releases/tag/v0.1.1
