use micropub::client::{MicropubAction, MicropubRequest};
use serde_json::json;

#[test]
fn test_create_request_json() {
    let mut props = serde_json::Map::new();
    props.insert("content".to_string(), json!(["Hello world"]));

    let req = MicropubRequest {
        action: MicropubAction::Create,
        properties: props,
        url: None,
    };

    let json = req.to_json().expect("Should serialize");
    assert!(json.contains("content"));
    assert!(json.contains("Hello world"));
}
