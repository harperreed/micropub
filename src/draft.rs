// ABOUTME: Draft management for micropub CLI
// ABOUTME: Handles draft creation, parsing, serialization, and lifecycle

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use is_terminal::IsTerminal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::config::{get_archive_dir, get_drafts_dir, Config};
use crate::draft_push::validate_draft_id;

/// Helper function to prompt user for showing more results
fn prompt_for_more() -> Result<bool> {
    if !io::stdout().is_terminal() {
        return Ok(false);
    }

    print!("Show more results? [y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DraftMetadata {
    #[serde(rename = "type")]
    pub post_type: String,
    pub name: Option<String>,
    pub published: Option<DateTime<Utc>>,
    #[serde(default)]
    pub category: Vec<String>,
    #[serde(default)]
    pub syndicate_to: Vec<String>,
    pub profile: Option<String>,
    #[serde(default)]
    pub photo: Vec<String>,
    pub status: Option<String>,
    pub url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}

impl Default for DraftMetadata {
    fn default() -> Self {
        Self {
            post_type: "note".to_string(),
            name: None,
            published: None,
            category: Vec::new(),
            syndicate_to: Vec::new(),
            profile: None,
            photo: Vec::new(),
            status: None,
            url: None,
            published_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Draft {
    pub id: String,
    pub metadata: DraftMetadata,
    pub content: String,
}

impl Draft {
    /// Create a new draft with default metadata
    pub fn new(id: String) -> Self {
        Self {
            id,
            metadata: DraftMetadata::default(),
            content: String::new(),
        }
    }

    /// Parse a draft from a string (YAML frontmatter + content)
    pub fn from_string(id: String, source: String) -> Result<Self> {
        // Split on --- delimiters
        let parts: Vec<&str> = source.splitn(3, "---").collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid draft format: missing frontmatter delimiters");
        }

        let frontmatter = parts[1].trim();
        let content = parts[2].trim().to_string();

        let metadata: DraftMetadata =
            serde_yaml::from_str(frontmatter).context("Failed to parse frontmatter")?;

        Ok(Self {
            id,
            metadata,
            content,
        })
    }

    /// Load a draft from file
    pub fn load(id: &str) -> Result<Self> {
        // Validate draft ID to prevent path traversal
        validate_draft_id(id)?;

        let path = get_drafts_dir()?.join(format!("{}.md", id));
        let contents = fs::read_to_string(&path).context("Failed to read draft file")?;
        Self::from_string(id.to_string(), contents)
    }

    /// Serialize draft to string (YAML frontmatter + content)
    pub fn to_string(&self) -> Result<String> {
        let frontmatter =
            serde_yaml::to_string(&self.metadata).context("Failed to serialize frontmatter")?;

        Ok(format!("---\n{}---\n\n{}", frontmatter, self.content))
    }

    /// Save draft to file
    pub fn save(&self) -> Result<PathBuf> {
        let path = get_drafts_dir()?.join(format!("{}.md", self.id));
        let contents = self.to_string()?;
        fs::write(&path, contents).context("Failed to write draft file")?;
        Ok(path)
    }

    /// Archive this draft (move to archive directory)
    pub fn archive(&self) -> Result<PathBuf> {
        let archive_path = get_archive_dir()?.join(format!("{}.md", self.id));
        let contents = self.to_string()?;
        fs::write(&archive_path, contents).context("Failed to write archived draft")?;

        // Remove from drafts directory
        let draft_path = get_drafts_dir()?.join(format!("{}.md", self.id));
        if draft_path.exists() {
            fs::remove_file(&draft_path)?;
        }

        Ok(archive_path)
    }

    /// List all draft IDs
    pub fn list_all() -> Result<Vec<String>> {
        let drafts_dir = get_drafts_dir()?;
        let mut draft_ids = Vec::new();

        for entry in fs::read_dir(drafts_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    draft_ids.push(stem.to_string());
                }
            }
        }

        Ok(draft_ids)
    }
}

/// Generate a new draft ID
pub fn generate_draft_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create a new draft and open in editor
pub fn cmd_new() -> Result<()> {
    let id = generate_draft_id();
    let draft = Draft::new(id.clone());

    // Save initial draft
    let path = draft.save()?;

    // Open in editor
    let config = Config::load()?;
    let editor = config
        .editor
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vim".to_string());

    Command::new(&editor)
        .arg(&path)
        .status()
        .context("Failed to open editor")?;

    println!("Draft created: {}", id);
    println!("Path: {}", path.display());

    Ok(())
}

/// Edit an existing draft
pub fn cmd_edit(draft_id: &str) -> Result<()> {
    // Validate draft ID to prevent path traversal
    validate_draft_id(draft_id)?;

    let path = get_drafts_dir()?.join(format!("{}.md", draft_id));

    if !path.exists() {
        anyhow::bail!("Draft not found: {}", draft_id);
    }

    let config = Config::load()?;
    let editor = config
        .editor
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vim".to_string());

    Command::new(&editor)
        .arg(&path)
        .status()
        .context("Failed to open editor")?;

    Ok(())
}

/// List all drafts with optional category filter
pub fn cmd_list(category_filter: Option<&str>, limit: usize, offset: usize) -> Result<()> {
    let mut all_draft_ids = Draft::list_all()?;

    if all_draft_ids.is_empty() {
        println!("No drafts found.");
        return Ok(());
    }

    // Sort for consistent ordering
    all_draft_ids.sort();

    // Apply category filter first to get filtered list
    let filtered_drafts: Vec<_> = if let Some(filter) = category_filter {
        all_draft_ids
            .into_iter()
            .filter_map(|id| {
                Draft::load(&id).ok().and_then(|draft| {
                    if draft.metadata.category.iter().any(|c| c == filter) {
                        Some((id, draft))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        all_draft_ids
            .into_iter()
            .filter_map(|id| Draft::load(&id).ok().map(|draft| (id, draft)))
            .collect()
    };

    if filtered_drafts.is_empty() {
        if category_filter.is_some() {
            println!("No drafts found with that category.");
        } else {
            println!("No drafts found.");
        }
        return Ok(());
    }

    let mut current_offset = offset;
    let mut first_page = true;

    loop {
        let page_items: Vec<_> = filtered_drafts
            .iter()
            .skip(current_offset)
            .take(limit)
            .collect();

        if page_items.is_empty() {
            if first_page {
                println!("No drafts found at offset {}.", current_offset);
            } else {
                println!("No more drafts.");
            }
            return Ok(());
        }

        if first_page {
            if let Some(filter) = category_filter {
                println!("Drafts with category '{}':", filter);
            } else {
                println!("Drafts:");
            }
        }

        for (id, draft) in page_items {
            let title = draft.metadata.name.as_deref().unwrap_or("[untitled]");
            let post_type = &draft.metadata.post_type;
            let categories = if draft.metadata.category.is_empty() {
                String::new()
            } else {
                format!(" [{}]", draft.metadata.category.join(", "))
            };
            println!("  {} - {} ({}){}", id, title, post_type, categories);
        }

        // Check if there are more results
        let remaining = filtered_drafts.len().saturating_sub(current_offset + limit);
        if remaining == 0 {
            // No more results
            return Ok(());
        }

        // Prompt for more results if in TTY mode
        if !prompt_for_more()? {
            return Ok(());
        }

        current_offset += limit;
        first_page = false;
    }
}

/// Search drafts by content or metadata
pub fn cmd_search(query: &str) -> Result<()> {
    let draft_ids = Draft::list_all()?;

    if draft_ids.is_empty() {
        println!("No drafts found.");
        return Ok(());
    }

    let query_lower = query.to_lowercase();
    let mut found_count = 0;

    println!("Searching for '{}'...\n", query);

    for id in draft_ids {
        match Draft::load(&id) {
            Ok(draft) => {
                let mut matches = Vec::new();

                // Search in title
                if let Some(ref title) = draft.metadata.name {
                    if title.to_lowercase().contains(&query_lower) {
                        matches.push("title");
                    }
                }

                // Search in content
                if draft.content.to_lowercase().contains(&query_lower) {
                    matches.push("content");
                }

                // Search in categories
                if draft
                    .metadata
                    .category
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query_lower))
                {
                    matches.push("category");
                }

                if !matches.is_empty() {
                    found_count += 1;
                    let title = draft
                        .metadata
                        .name
                        .unwrap_or_else(|| "[untitled]".to_string());
                    println!("{} - {}", id, title);
                    println!("  Matched in: {}", matches.join(", "));

                    // Show a snippet of content if it matched
                    if matches.contains(&"content") {
                        let snippet = draft
                            .content
                            .lines()
                            .find(|line| line.to_lowercase().contains(&query_lower))
                            .map(|line| {
                                if line.len() > 80 {
                                    format!("{}...", &line[..77])
                                } else {
                                    line.to_string()
                                }
                            })
                            .unwrap_or_default();
                        if !snippet.is_empty() {
                            println!("  {}", snippet);
                        }
                    }
                    println!();
                }
            }
            Err(_) => continue,
        }
    }

    if found_count == 0 {
        println!("No drafts found matching '{}'.", query);
    } else {
        println!("Found {} draft(s).", found_count);
    }

    Ok(())
}

/// Show a draft's content
pub fn cmd_show(draft_id: &str) -> Result<()> {
    // Validate draft ID to prevent path traversal
    validate_draft_id(draft_id)?;

    let draft = Draft::load(draft_id)?;
    println!("{}", draft.to_string()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_roundtrip() {
        let original = Draft {
            id: "test".to_string(),
            metadata: DraftMetadata {
                post_type: "article".to_string(),
                name: Some("Test Post".to_string()),
                category: vec!["test".to_string()],
                ..Default::default()
            },
            content: "Test content".to_string(),
        };

        let serialized = original.to_string().unwrap();
        let parsed = Draft::from_string("test".to_string(), serialized).unwrap();

        assert_eq!(parsed.metadata.name, original.metadata.name);
        assert_eq!(parsed.content, original.content);
    }
}
