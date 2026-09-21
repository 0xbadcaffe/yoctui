use super::*;

#[test]
fn raw_preview_includes_optional_joined_value_and_preserves_explicit_empty() {
    let mut request = request();
    request
        .parameters
        .insert(id("ui"), RawParameterValue::UserInterface("knotty".into()));
    let preview = catalog()
        .preview(&request, Some(&authority(9, true)))
        .unwrap();
    assert_eq!(preview.arguments[2], "--ui=knotty");
    assert!(preview.arguments.iter().any(String::is_empty));
}
