# Micropub CLI Usage Guide

## Configuration

Configuration is stored in `~/.config/micropub/config.toml`:

```toml
default_profile = "micro.blog"
editor = "vim"

[profiles.micro.blog]
domain = "micro.blog"
micropub_endpoint = "https://micro.blog/micropub"
media_endpoint = "https://micro.blog/micropub/media"
```

Tokens are stored separately in `~/.local/share/micropub/tokens/`.

## Authentication

Authenticate with a Micropub site:

```bash
micropub auth micro.blog
```

This will:
1. Discover micropub and authorization endpoints
2. Prompt for an API token
3. Save the profile and token

## Draft Management

### Create a new draft

```bash
micropub draft new
```

Opens your editor with a new draft template.

### Draft format

```markdown
---
type: article
name: "My Post Title"
category:
  - rust
  - blogging
syndicate-to:
  - https://twitter.com/username
---

Post content goes here.

You can reference local images: ![photo](~/Pictures/image.jpg)
```

### List drafts

```bash
micropub draft list
```

### Edit a draft

```bash
micropub draft edit <draft-id>
```

### Show draft content

```bash
micropub draft show <draft-id>
```

## Publishing

### Publish a draft

```bash
micropub publish <draft-id>
```

This will:
1. Parse the draft
2. Upload any media files
3. Replace local paths with URLs
4. Send to micropub endpoint
5. Archive the draft with publication metadata

### Backdate a post

```bash
micropub backdate <draft-id> --date "2024-01-15T10:30:00Z"
```

## Post Management

### Delete a post

```bash
micropub delete <post-url>
```

### Undelete a post

```bash
micropub undelete <post-url>
```

### Update a post

```bash
micropub update <post-url>
```

(Coming soon)

## Multi-Site Usage

### Use a specific profile

Add `profile: mysite` to draft frontmatter, or use `--profile` flag:

```bash
micropub publish <draft-id> --profile mysite
```

## Troubleshooting

### Debug connection

```bash
micropub debug <profile-name>
```

### Check configuration

```bash
cat ~/.config/micropub/config.toml
```

### Verify token

```bash
cat ~/.local/share/micropub/tokens/<profile>.token
```
