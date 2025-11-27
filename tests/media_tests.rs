use micropub::media::find_media_references;

#[test]
fn test_find_markdown_images() {
    let content = "Here's an image: ![alt](~/photo.jpg) and another ![](./pic.png)";
    let refs = find_media_references(content);

    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"~/photo.jpg".to_string()));
    assert!(refs.contains(&"./pic.png".to_string()));
}

#[test]
fn test_find_html_images() {
    let content = r#"<img src="~/image.png"> and <img src="/abs/path.jpg">"#;
    let refs = find_media_references(content);

    assert_eq!(refs.len(), 2);
}
