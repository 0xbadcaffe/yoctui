use super::*;

#[test]
fn batch_datastore_is_exact_bounded_and_preserves_paths() {
    let mut values = VARIABLES
        .into_iter()
        .map(|key| (key.to_owned(), serde_json::Value::Null))
        .collect::<serde_json::Map<_, _>>();
    values.insert("MACHINE".into(), "romulus".into());
    values.insert("BBLAYERS".into(), "/src/meta /src/meta-openbmc".into());
    let report = serde_json::json!({"build_directory": "/build", "values": values});
    let result = parse(&report.to_string(), Path::new("/build")).unwrap();
    assert_eq!(result["MACHINE"], "romulus");
    assert_eq!(result["BBLAYERS"], "/src/meta /src/meta-openbmc");
    assert!(parse(&report.to_string(), Path::new("/other")).is_err());
    assert!(parse("{}", Path::new("/build")).is_err());
    let mut report = report;
    report["values"]["EXTRA"] = "unexpected".into();
    assert!(parse(&report.to_string(), Path::new("/build")).is_err());
}
