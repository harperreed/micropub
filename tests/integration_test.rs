use micropub::draft::{Draft, generate_draft_id};
use micropub::config::Config;

#[test]
fn test_draft_lifecycle() {
    let id = generate_draft_id();
    let mut draft = Draft::new(id.clone());
    draft.metadata.name = Some("Test Post".to_string());
    draft.content = "Test content here".to_string();

    // Save
    let path = draft.save().expect("Should save draft");
    assert!(path.exists());

    // Load
    let loaded = Draft::load(&id).expect("Should load draft");
    assert_eq!(loaded.metadata.name, Some("Test Post".to_string()));
    assert_eq!(loaded.content, "Test content here");

    // Archive
    let archive_path = loaded.archive().expect("Should archive");
    assert!(archive_path.exists());
    assert!(!path.exists()); // Original should be removed
}

#[test]
fn test_config_roundtrip() {
    use std::collections::HashMap;
    use micropub::config::Profile;

    let mut config = Config {
        default_profile: "test".to_string(),
        editor: Some("vim".to_string()),
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

    config.save().expect("Should save config");

    let loaded = Config::load().expect("Should load config");
    assert_eq!(loaded.default_profile, "test");
    assert!(loaded.get_profile("test").is_some());
}
