# MCP Image Upload Feature Design

**Date:** 2025-12-03
**Status:** Approved
**Problem:** Agents using the Micropub MCP server think they cannot upload images because the capability is not exposed as an MCP tool.

## Architecture Overview

We'll add image upload capabilities to the MCP server through two complementary approaches:

### 1. Explicit Upload Tool (`upload_media`)
- New MCP tool that agents call directly
- Takes file path OR base64 data with optional alt text
- Returns structured JSON with url, filename, mime_type, and ready-to-use markdown
- Leverages existing `media::upload_file()` function

### 2. Enhanced Automatic Upload (existing `publish_post`)
- Already works: detects local paths like `![](~/photo.jpg)` in content
- We'll enhance visibility through better documentation and feedback
- Returns information about what was uploaded in the response

### 3. Discoverability Improvements
- Update `publish_post` tool description to mention auto-upload
- Modify response messages to confirm uploads with details
- Add server-level instructions explaining both upload methods

This design reuses the battle-tested `media::upload_file()` function (lines 74-122 in media.rs) which already handles multipart uploads, MIME type detection, and Location header parsing.

## New `upload_media` Tool Implementation

### Tool Parameters

```rust
pub struct UploadMediaArgs {
    /// Path to local file (e.g., ~/Pictures/photo.jpg)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Base64-encoded file data (alternative to file_path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,

    /// Filename (required when using file_data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Optional alt text for accessibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
}
```

### Validation Logic
- Must provide EITHER `file_path` OR (`file_data` + `filename`)
- If `file_data` provided, `filename` is required
- Alt text is always optional, defaults to empty string in markdown

### Return Format

JSON string with all metadata:
```json
{
  "url": "https://example.com/media/abc123.jpg",
  "filename": "sunset.jpg",
  "mime_type": "image/jpeg",
  "markdown": "![Beautiful sunset](https://example.com/media/abc123.jpg)"
}
```

### Implementation Flow
1. Validate parameters (either path OR data+filename)
2. If base64 data: decode to temp file
3. Call `media::upload_file()` with endpoint/token from config
4. Build structured response with all metadata
5. Return as JSON text content

## Enhanced `publish_post` Feedback

### Current Behavior (already working)
- `publish_post` calls `find_media_references()` to detect local paths
- Automatically uploads files via `upload_file()`
- Replaces local paths with URLs in content
- This happens in `cmd_publish()` (lines 58-82 in publish.rs)

### Changes

#### 1. Enhanced Tool Description
```rust
#[tool(description = "Create and publish a micropub post with optional title and categories. Automatically detects and uploads local image files (e.g., ![alt](~/photo.jpg) or <img src='/path/image.png'>) and replaces them with permanent URLs before publishing.")]
```

#### 2. Modified Return Message
Instead of just "Post published successfully!", return detailed feedback:
```
Post published successfully!

Uploaded media:
- sunset.jpg -> https://example.com/media/abc123.jpg
- beach.png -> https://example.com/media/def456.png
```

#### 3. Track Uploads in `cmd_publish()`
- Collect upload details (filename, URL) during the upload loop
- Return this information to the MCP tool handler
- Modify `cmd_publish()` to return `Result<Vec<(String, String)>>` (filename, url pairs)

## Discoverability Strategy

### 1. Server-Level Instructions Update

Modify `get_info()` implementation (lines 845-858 in mcp.rs):

```rust
instructions: Some(
    "Micropub MCP server for posting and managing micropub content via AI assistants.\n\n\
     IMAGE UPLOADS:\n\
     - Use 'upload_media' tool to upload images explicitly (supports file paths or base64 data)\n\
     - Or use 'publish_post' with local image paths (e.g., ![alt](~/photo.jpg)) - they'll auto-upload\n\n\
     All uploads require authentication via 'micropub auth <domain>' first."
        .to_string(),
),
```

### 2. Photo-Post Prompt Enhancement

Update the `photo_post` prompt (lines 609-646):

```rust
PromptMessage::new_text(
    PromptMessageRole::Assistant,
    format!(
        "I'll help you create a photo post about {}. You can:\n\
         1. Upload the image first with 'upload_media' tool, then use the URL\n\
         2. Reference a local file (e.g., ~/Pictures/photo.jpg) and I'll auto-upload when publishing\n\n\
         Please provide the photo path and a caption.",
        subject
    ),
),
```

### 3. Tool Metadata

Add clear examples to the `upload_media` tool description showing both usage modes.

## Error Handling

### `upload_media` Tool Errors
- **Missing media endpoint:** "No media endpoint configured. Server may not support media uploads."
- **Invalid parameters:** "Must provide either file_path OR (file_data + filename)"
- **File not found:** "File not found: {path}"
- **Base64 decode failure:** "Invalid base64 data"
- **Upload failure:** Propagate error from `upload_file()` with status code and server response
- **Missing auth:** "No authentication token found. Run 'micropub auth <domain>' first"

### `publish_post` Enhancement Errors
- Non-fatal: If media upload fails, still show error but don't block post publication
- Return both success message AND upload errors in output
- Example: "Post published! Warning: Failed to upload beach.png: File not found"

## Testing Strategy

### Unit Tests (tests/mcp_tests.rs)
- Test `upload_media` parameter validation
- Test base64 decoding with sample image data
- Test structured response JSON format
- Mock the upload endpoint to avoid real HTTP calls

### Integration Tests
- Test automatic upload detection in content
- Test path resolution (~/path, relative, absolute)
- Test markdown and HTML image syntax detection
- Test response message formatting with multiple uploads

### Manual Testing
- Upload via file path with Claude Desktop
- Upload via base64 data
- Test auto-upload in `publish_post` with mixed local/remote images
- Verify markdown output with alt text

## Files to Modify

1. **src/mcp.rs**
   - Add `UploadMediaArgs` struct
   - Add `upload_media` tool implementation
   - Update `publish_post` description
   - Enhance `publish_post` return message
   - Update server instructions in `get_info()`
   - Update `photo_post` prompt

2. **src/publish.rs**
   - Modify `cmd_publish()` signature to return upload details
   - Track uploaded files in the upload loop
   - Return `Result<Vec<(String, String)>>`

3. **tests/mcp_tests.rs**
   - Add unit tests for `upload_media` validation
   - Add tests for response format
   - Add tests for enhanced `publish_post` feedback

## Implementation Notes

- Reuse existing `media::upload_file()` - no changes needed to core upload logic
- Base64 uploads should use temp files (via `tempfile` crate) to avoid memory issues with large images
- Temp files should be cleaned up automatically when dropped
- JSON serialization for response uses `serde_json::to_string()`
- Alt text in markdown uses standard format: `![alt text](url)`
