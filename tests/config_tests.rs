use micropub::config::{get_config_dir, get_data_dir};
use std::path::PathBuf;

#[test]
fn test_config_dir_exists() {
    let config_dir = get_config_dir().expect("Should get config dir");
    assert!(config_dir.to_str().unwrap().contains("micropub"));
}

#[test]
fn test_data_dir_exists() {
    let data_dir = get_data_dir().expect("Should get data dir");
    assert!(data_dir.to_str().unwrap().contains("micropub"));
}
