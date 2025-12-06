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
