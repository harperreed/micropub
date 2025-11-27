use micropub::draft::{Draft, DraftMetadata};

#[test]
fn test_parse_draft_with_frontmatter() {
    let content = r#"---
type: article
name: "Test Post"
category:
  - rust
  - micropub
---

This is the post content.
"#;

    let draft = Draft::from_string("test-id".to_string(), content.to_string())
        .expect("Should parse draft");

    assert_eq!(draft.metadata.name, Some("Test Post".to_string()));
    assert_eq!(draft.metadata.category, vec!["rust", "micropub"]);
    assert_eq!(draft.content.trim(), "This is the post content.");
}

#[test]
fn test_draft_to_string() {
    let mut draft = Draft::new("test-id".to_string());
    draft.metadata.name = Some("Test".to_string());
    draft.content = "Content here".to_string();

    let output = draft.to_string().expect("Should serialize");
    assert!(output.contains("name: Test"));
    assert!(output.contains("Content here"));
}
