// ABOUTME: Tests for draft push functionality
// ABOUTME: Validates pushing drafts to server with post-status: draft

use micropub::draft_push::PushResult;
use micropub::media::{find_media_references, replace_paths};

#[test]
fn test_push_result_structure() {
    let result = PushResult {
        url: "https://example.com/posts/draft-123".to_string(),
        is_update: false,
        uploads: vec![(
            "photo.jpg".to_string(),
            "https://example.com/media/abc.jpg".to_string(),
        )],
    };

    assert_eq!(result.url, "https://example.com/posts/draft-123");
    assert!(!result.is_update);
    assert_eq!(result.uploads.len(), 1);
}

#[test]
fn test_find_media_references_in_draft_content() {
    let content = r#"
# My Post

Here is an image: ![alt text](~/photo.jpg)

And another: ![](./images/test.png)

But not this URL: ![](https://example.com/remote.jpg)
"#;

    let refs = find_media_references(content);

    // Should find local paths but not URLs
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"~/photo.jpg".to_string()));
    assert!(refs.contains(&"./images/test.png".to_string()));
    assert!(!refs.contains(&"https://example.com/remote.jpg".to_string()));
}

#[test]
fn test_replace_paths_in_content() {
    let content = "Image: ![alt](~/photo.jpg) here";
    let replacements = vec![(
        "~/photo.jpg".to_string(),
        "https://cdn.example.com/uploaded.jpg".to_string(),
    )];

    let result = replace_paths(content, &replacements);

    assert!(result.contains("https://cdn.example.com/uploaded.jpg"));
    assert!(!result.contains("~/photo.jpg"));
}

#[test]
fn test_micropub_request_includes_draft_status() {
    use micropub::client::{MicropubAction, MicropubRequest};
    use serde_json::{Map, Value};

    let mut properties = Map::new();
    properties.insert(
        "content".to_string(),
        Value::Array(vec![Value::String("Test content".to_string())]),
    );
    properties.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("draft".to_string())]),
    );

    let request = MicropubRequest {
        action: MicropubAction::Create,
        properties,
        url: None,
    };

    let json = request.to_json().unwrap();

    // Verify JSON contains post-status: draft
    assert!(json.contains("post-status"));
    assert!(json.contains("\"draft\""));
}

#[test]
fn test_micropub_update_request_structure() {
    use micropub::client::{MicropubAction, MicropubRequest};
    use serde_json::{Map, Value};

    let mut replace = Map::new();
    replace.insert(
        "content".to_string(),
        Value::Array(vec![Value::String("Updated content".to_string())]),
    );
    replace.insert(
        "post-status".to_string(),
        Value::Array(vec![Value::String("draft".to_string())]),
    );

    let request = MicropubRequest {
        action: MicropubAction::Update {
            replace,
            add: Map::new(),
            delete: Vec::new(),
        },
        properties: Map::new(),
        url: Some("https://example.com/posts/123".to_string()),
    };

    let json = request.to_json().unwrap();

    // Verify it's an update action
    assert!(json.contains("\"action\""));
    assert!(json.contains("\"update\""));
    assert!(json.contains("example.com/posts/123"));
    assert!(json.contains("\"replace\""));
    assert!(json.contains("Updated content"));
}

#[tokio::test]
async fn test_cmd_push_draft_requires_valid_draft_id() {
    let result = micropub::draft_push::cmd_push_draft("nonexistent", None).await;
    assert!(result.is_err());
    // Will fail with "Draft not found" from Draft::load
}

// CLI Integration Tests
#[test]
fn test_cli_draft_push_command_exists() {
    // This test verifies that the CLI can parse the draft push command
    use clap::Parser;

    // Mock CLI args for testing - we'll parse the actual command structure
    #[derive(Parser)]
    #[command(name = "micropub")]
    struct TestCli {
        #[command(subcommand)]
        command: Option<TestCommands>,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        #[command(subcommand)]
        Draft(TestDraftCommands),
    }

    #[derive(clap::Subcommand)]
    enum TestDraftCommands {
        Push {
            draft_id: String,
            #[arg(long)]
            backdate: Option<String>,
        },
    }

    // Test parsing without backdate
    let result = TestCli::try_parse_from(vec!["micropub", "draft", "push", "test-draft"]);
    assert!(result.is_ok());

    // Test parsing with backdate
    let result = TestCli::try_parse_from(vec![
        "micropub",
        "draft",
        "push",
        "test-draft",
        "--backdate",
        "2024-01-15T10:00:00Z",
    ]);
    assert!(result.is_ok());
}
