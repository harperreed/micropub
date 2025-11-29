use micropub::config::Config;
use micropub::draft::{generate_draft_id, Draft};

#[test]
#[ignore] // DISABLED: Test writes to production data directory - needs refactoring to use temp dirs
fn test_draft_lifecycle() {
    // TODO: Refactor draft module to support dependency injection of data directory path
    // This test currently pollutes production drafts/archive and should not be run

    let id = generate_draft_id();
    let mut draft = Draft::new(id.clone());
    draft.metadata.name = Some("Test Post".to_string());
    draft.content = "Test content here".to_string();

    // Verify in-memory operations work
    assert_eq!(draft.metadata.name, Some("Test Post".to_string()));
    assert_eq!(draft.content, "Test content here");

    // Cannot test save/load/archive without polluting production directories
    // These operations write to ~/Library/Application Support/micropub/drafts/
}

#[test]
#[ignore] // DISABLED: Test writes to production config file - needs refactoring to use temp dirs
fn test_config_roundtrip() {
    use micropub::config::Profile;
    use std::collections::HashMap;

    // TODO: Refactor config module to support dependency injection of config path
    // This test currently pollutes production config and should not be run

    let mut config = Config {
        default_profile: "test".to_string(),
        editor: Some("vim".to_string()),
        client_id: None,
        profiles: HashMap::new(),
    };

    config.upsert_profile(
        "test".to_string(),
        Profile {
            domain: "example.com".to_string(),
            micropub_endpoint: Some("https://example.com/micropub".to_string()),
            media_endpoint: None,
            token_endpoint: None,
            authorization_endpoint: None,
        },
    );

    // This would write to production: config.save().expect("Should save config");

    // Verify in-memory operations work
    assert_eq!(config.default_profile, "test");
    assert!(config.get_profile("test").is_some());
}
