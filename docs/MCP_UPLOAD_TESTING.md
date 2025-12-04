# MCP Image Upload Testing Guide

## Prerequisites
- Authenticated micropub profile: `micropub auth <your-domain.com>`
- Test image file: `~/test-image.jpg`

## Test 1: upload_media with File Path

**MCP Tool Call:**
```json
{
  "tool": "upload_media",
  "arguments": {
    "file_path": "~/test-image.jpg",
    "alt_text": "Test image"
  }
}
```

**Expected Response:**
```json
{
  "url": "https://yourdomain.com/media/abc123.jpg",
  "filename": "test-image.jpg",
  "mime_type": "image/jpeg",
  "markdown": "![Test image](https://yourdomain.com/media/abc123.jpg)"
}
```

## Test 2: upload_media with Base64

**Preparation:**
```bash
base64 -i ~/test-image.jpg | pbcopy
```

**MCP Tool Call:**
```json
{
  "tool": "upload_media",
  "arguments": {
    "file_data": "<paste base64 here>",
    "filename": "test-image.jpg"
  }
}
```

**Expected:** Same response format with uploaded URL

## Test 3: publish_post with Auto-Upload

**MCP Tool Call:**
```json
{
  "tool": "publish_post",
  "arguments": {
    "content": "Check out this photo! ![My test](~/test-image.jpg)",
    "title": "Photo Test"
  }
}
```

**Expected Response:**
```
Post published successfully!

Uploaded media:
- test-image.jpg -> https://yourdomain.com/media/abc123.jpg
```

## Test 4: Error Cases

**No file_path or file_data:**
```json
{"tool": "upload_media", "arguments": {}}
```
Expected: "Must provide either file_path OR file_data"

**file_data without filename:**
```json
{"tool": "upload_media", "arguments": {"file_data": "abc123"}}
```
Expected: "filename is required when using file_data"

**Both file_path and file_data:**
```json
{
  "tool": "upload_media",
  "arguments": {
    "file_path": "~/test.jpg",
    "file_data": "abc123",
    "filename": "test.jpg"
  }
}
```
Expected: "Cannot provide both file_path and file_data"

## Test 5: Server Instructions Visibility

**MCP Server Info Request:**
Check that instructions field contains upload guidance.

Expected to see:
```
IMAGE UPLOADS:
- Use 'upload_media' tool to upload images explicitly (supports file paths or base64 data)
- Or use 'publish_post' with local image paths (e.g., ![alt](~/photo.jpg)) - they'll auto-upload
```

## Verification Checklist

- [ ] upload_media with file path works
- [ ] upload_media with base64 data works
- [ ] Alt text appears in markdown output
- [ ] publish_post auto-uploads local images
- [ ] publish_post shows upload feedback
- [ ] Error messages are clear and helpful
- [ ] Server instructions mention both upload methods
- [ ] Photo-post prompt mentions both methods
