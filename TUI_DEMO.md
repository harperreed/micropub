# Micropub TUI Demo

## Launching the TUI

### When Not Authenticated
```bash
$ micropub
Welcome to Micropub CLI!

To get started, authenticate with your site:
  micropub auth <your-domain.com>

For more help, run:
  micropub --help
```

### When Authenticated
```bash
$ micropub
# Launches full-screen TUI interface
```

## TUI Interface Screenshots (ASCII Art)

### Drafts View (Default)
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Drafts (3)               │ Preview                                     │
│                          │                                             │
│ > Test Draft for TUI     │ ---                                         │
│   (note) [test, demo]    │ type: note                                  │
│                          │ name: "Test Draft for TUI"                  │
│   My Vacation to Beach   │ category:                                   │
│   (article) [travel]     │   - test                                    │
│                          │   - demo                                    │
│   Test Article           │ ---                                         │
│   (article) [testing]    │                                             │
│                          │ This is a test draft to demonstrate the TUI │
│                          │ functionality.                              │
│                          │                                             │
│                          │ It has some content that should be visible  │
│                          │ in the preview pane.                        │
│                          │                                             │
│                          │                                             │
├──────────────────────────┴─────────────────────────────────────────────┤
│ [p]ublish [e]dit [d]elete [n]ew [r]efresh [q]uit                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### Posts View
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Posts                    │ Info                                        │
│                          │                                             │
│ Posts view coming soon...│ Posts view - coming soon                    │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
├──────────────────────────┴─────────────────────────────────────────────┤
│ [r]efresh [q]uit                                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### Media View
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Media                    │ Info                                        │
│                          │                                             │
│ Media view coming soon...│ Media view - coming soon                    │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
├──────────────────────────┴─────────────────────────────────────────────┤
│ [r]efresh [q]uit                                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### Confirmation Dialog
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Drafts (3)               │ Preview                                     │
│                          │                                             │
│ > Test Draft for TUI     │ ---                                         │
│   (note) [test, demo]    │ type: note                                  │
│                          │ name: "Test Draft for TUI"                  │
│   My Vacation to Beach   │ category:                                   │
│   (article) [travel]     │   - test                                    │
│                          │   - demo                                    │
│   Test Article           │ ---                                         │
│   (article) [testing]    │                                             │
│                          │ This is a test draft to demonstrate the TUI │
│                          │ functionality.                              │
│                          │                                             │
├──────────────────────────┴─────────────────────────────────────────────┤
│ Status: Publish draft? (y/n)                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### After Publishing
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Drafts (2)               │ Preview                                     │
│                          │                                             │
│ > My Vacation to Beach   │ ---                                         │
│   (article) [travel]     │ type: article                               │
│                          │ name: "My Vacation to the Beach"            │
│   Test Article           │ category:                                   │
│   (article) [testing]    │   - travel                                  │
│                          │   - personal                                │
│                          │ ---                                         │
│                          │                                             │
│                          │ I had a wonderful time at the beach last    │
│                          │ week. The weather was perfect!              │
│                          │                                             │
│                          │ Here are some highlights:                   │
│                          │ - Beautiful sunset                          │
│                          │ - Great seafood                             │
│                          │ - Relaxing atmosphere                       │
├──────────────────────────┴─────────────────────────────────────────────┤
│ Status: Draft published successfully!                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Error Display
```
┌─ Micropub Manager ─────────────────────────────────────────────────────┐
│ [1] Drafts  [2] Posts  [3] Media                                       │
├──────────────────────────┬─────────────────────────────────────────────┤
│ Drafts (2)               │ Preview                                     │
│                          │                                             │
│ > My Vacation to Beach   │ ...                                         │
│   (article) [travel]     │                                             │
│                          │                                             │
│   Test Article           │                                             │
│   (article) [testing]    │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
│                          │                                             │
├──────────────────────────┴─────────────────────────────────────────────┤
│ Error: Edit mode not supported in TUI. Use 'micropub draft edit <id>'  │
│ [Esc] to dismiss                                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

## Keyboard Reference

### Global Keys
- `q` - Quit (press twice to confirm)
- `Tab` - Next tab
- `Shift+Tab` - Previous tab
- `Esc` - Clear error/status messages

### Navigation
- `j` or `↓` - Next item
- `k` or `↑` - Previous item
- `Enter` - Select item

### Drafts View Actions
- `p` - Publish selected draft (requires confirmation)
- `d` - Delete selected draft (requires confirmation)
- `r` - Refresh draft list
- `e` - Edit draft (shows error - use CLI)
- `n` - New draft (shows error - use CLI)
- `b` - Backdate draft (shows error - use CLI)

### Confirmation Keys
- `y` - Confirm action
- `n` - Cancel action

## Usage Examples

### Browse Drafts
1. Launch micropub with no arguments: `micropub`
2. Use `j`/`k` or arrow keys to navigate
3. Preview content appears automatically in right pane

### Publish a Draft
1. Navigate to the draft you want to publish
2. Press `p`
3. Press `y` to confirm (or `n` to cancel)
4. Wait for "Draft published successfully!" message
5. Draft is removed from list and archived

### Delete a Draft
1. Navigate to the draft you want to delete
2. Press `d`
3. Press `y` to confirm (or `n` to cancel)
4. Draft is removed from list

### Switch Between Views
1. Press `Tab` to cycle through: Drafts → Posts → Media → Drafts
2. Press `Shift+Tab` to cycle backwards

### Refresh View
1. Press `r` to reload the current view
2. Useful after creating drafts via CLI

## Integration with CLI

The TUI and CLI work seamlessly together:

```bash
# Create a draft via CLI
micropub draft new

# Launch TUI to browse and publish
micropub

# After publishing in TUI, view posts via CLI
micropub posts --limit 5

# Edit a draft via CLI
micropub draft edit <draft-id>

# Return to TUI to continue browsing
micropub
```

## Tips

1. **Quick Preview**: The preview updates automatically as you navigate
2. **Double-Quit**: Press `q` twice to exit (prevents accidental quits)
3. **Error Recovery**: Press `Esc` to clear any error messages
4. **CLI Fallback**: Use CLI commands for features not yet in TUI (edit, new, backdate)
5. **Refresh Often**: Press `r` after creating drafts via CLI
