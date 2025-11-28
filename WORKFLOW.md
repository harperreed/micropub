# Micropub CLI Workflows

## Backdated Post with Photo

This workflow shows how to publish a post dated in the past with an attached photo.

### Prerequisites

1. Authenticate with your micropub server:
   ```bash
   micropub auth your-domain.com
   ```

2. Verify authentication:
   ```bash
   micropub whoami
   ```

### Workflow

#### 1. Create a draft

```bash
micropub draft new
```

This opens your editor (set via `EDITOR` environment variable or defaults to system editor).

#### 2. Write your post with photo

In the draft file, use this format:

```yaml
---
title: "My Trip to the Beach"
category:
  - photos
  - travel
photo:
  - /path/to/beach-photo.jpg
---

Had an amazing day at the beach! The weather was perfect.
```

**Photo paths can be:**
- Absolute paths: `/Users/you/Pictures/photo.jpg`
- Relative paths: `./photos/beach.jpg` (relative to draft file)
- URLs: `https://example.com/photo.jpg` (will be downloaded)

**Multiple photos:**
```yaml
photo:
  - /path/to/photo1.jpg
  - /path/to/photo2.jpg
  - ./photos/photo3.jpg
```

Save and close the editor.

#### 3. Preview your draft

List all drafts to find the ID:

```bash
micropub draft list
```

View the draft content:

```bash
micropub draft show <draft-id>
```

Edit if needed:

```bash
micropub draft edit <draft-id>
```

#### 4. Publish with backdate

Publish with a specific date/time:

```bash
micropub backdate <draft-id> --date "2024-01-15T10:30:00Z"
```

**Date format:** ISO 8601 (RFC 3339)
- `2024-01-15T10:30:00Z` - UTC time
- `2024-01-15T10:30:00-08:00` - Pacific time
- `2024-01-15T10:30:00+01:00` - Central European time

**What happens:**
1. Photos are uploaded to your media endpoint (if configured)
2. The post is created with the photos and backdated timestamp
3. The draft is moved to the archive
4. You get the published URL

### Examples

#### Simple photo post from yesterday

```bash
# 1. Create draft
micropub draft new

# 2. Write content with photo
# ---
# photo:
#   - ~/Pictures/sunset.jpg
# ---
# Beautiful sunset today!

# 3. Publish backdated to yesterday at 6pm
micropub backdate <draft-id> --date "2024-01-20T18:00:00-08:00"
```

#### Trip photos from last month

```bash
# 1. Create draft
micropub draft new

# 2. Add multiple photos and content
# ---
# title: "Paris Trip"
# category:
#   - travel
#   - photos
# photo:
#   - ~/Pictures/paris/eiffel.jpg
#   - ~/Pictures/paris/louvre.jpg
#   - ~/Pictures/paris/cafe.jpg
# ---
# Amazing week in Paris! These are my favorite shots.

# 3. Backdate to the trip date
micropub backdate <draft-id> --date "2024-12-15T12:00:00+01:00"
```

#### Present-day post (no backdate)

If you just want to publish now without backdating:

```bash
micropub publish <draft-id>
```

This uses the current timestamp.

### Tips

1. **Draft IDs**: The draft ID is shown when you create a draft, or use `micropub draft list` to see all drafts

2. **Photo formats**: Supports common formats (JPEG, PNG, GIF, WebP) - your micropub server determines what's accepted

3. **Photo size**: Large photos are uploaded as-is - consider resizing beforehand if needed

4. **Archive**: Published drafts are automatically moved to `~/.local/share/micropub/archive/` (or `~/Library/Application Support/micropub/archive/` on macOS)

5. **Timezones**: Always include timezone in the date. Use `Z` for UTC or specify offset like `-08:00`

6. **List published posts**: See your recent posts with:
   ```bash
   micropub posts --limit 20
   ```

### Troubleshooting

**"Media endpoint not configured"**
- Your server doesn't support photo uploads, or it wasn't discovered during auth
- Re-authenticate: `micropub auth your-domain.com`

**"Invalid date format"**
- Use ISO 8601 format: `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DDTHH:MM:SS±HH:MM`
- Example: `2024-01-15T10:30:00-08:00`

**"Photo file not found"**
- Check the path is correct
- Use absolute paths to avoid confusion
- Ensure you have read permissions on the file

**"Token not found"**
- Run `micropub whoami` to check authentication
- Re-authenticate: `micropub auth your-domain.com`
