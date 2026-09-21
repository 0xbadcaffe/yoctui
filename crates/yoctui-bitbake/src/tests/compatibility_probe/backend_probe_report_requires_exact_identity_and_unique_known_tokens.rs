use super::*;

#[test]
fn backend_probe_report_requires_exact_identity_and_unique_known_tokens() {
    let report = serde_json::json!({
        "schema": "yoctui.bridge-capability-probe.v1",
        "build_directory": "/build/romulus",
        "bitbake_version": "2.19.0",
        "capabilities": ["workspace", "build", "native_events"],
    });
    let parse = |value: &serde_json::Value| {
        parse_backend_capabilities(&value.to_string(), Path::new("/build/romulus"), "2.19.0")
    };
    assert_eq!(parse(&report).unwrap().len(), 3);
    for (key, value) in [
        ("schema", serde_json::json!("future")),
        ("build_directory", serde_json::json!("/other")),
        ("bitbake_version", serde_json::json!("2.18.0")),
        ("capabilities", serde_json::json!(["build", "build"])),
        ("capabilities", serde_json::json!(["invented"])),
        ("capabilities", serde_json::json!(vec!["build"; 65])),
    ] {
        let mut invalid = report.clone();
        invalid[key] = value;
        assert!(parse(&invalid).is_err(), "{invalid}");
    }
    let mut empty = report.clone();
    empty["capabilities"] = serde_json::json!([]);
    assert!(parse(&empty).unwrap().is_empty());
    empty["unexpected"] = serde_json::json!(true);
    assert!(parse(&empty).is_err());
    assert!(parse_backend_capabilities("not json", Path::new("/build/romulus"), "2.19.0").is_err());
}
