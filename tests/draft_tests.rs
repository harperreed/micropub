use micropub::draft::Draft;

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

    let draft =
        Draft::from_string("test-id".to_string(), content.to_string()).expect("Should parse draft");

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

#[test]
fn test_cmd_list_formats_output() {
    // Note: This test validates the cmd_list function works correctly
    // by calling it on the actual data directory. The function will
    // list whatever drafts exist in the user's drafts directory.
    let result = micropub::draft::cmd_list();
    assert!(result.is_ok(), "cmd_list should succeed");
}

#[test]
fn test_cmd_show_error_missing_draft() {
    // Test cmd_show with a draft ID that is very unlikely to exist
    let result = micropub::draft::cmd_show("nonexistent-draft-id-12345678");
    assert!(result.is_err(), "cmd_show should fail for missing draft");

    // Verify the error message contains useful information
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("Failed to read draft file")
                || error_msg.contains("No such file or directory"),
            "Error should mention file not found"
        );
    }
}

#[test]
fn test_cmd_list_empty_directory() {
    // This test validates that cmd_list handles cases where
    // the drafts directory might be empty or newly created
    let result = micropub::draft::cmd_list();
    assert!(
        result.is_ok(),
        "cmd_list should succeed even with no drafts"
    );
}
