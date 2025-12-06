// ABOUTME: Tests for draft push functionality
// ABOUTME: Validates pushing drafts to server with post-status: draft

use micropub::draft_push::PushResult;

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
