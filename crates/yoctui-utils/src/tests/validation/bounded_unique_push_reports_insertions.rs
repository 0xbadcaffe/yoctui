use super::*;

#[test]
fn bounded_unique_push_reports_insertions() {
    let mut values = vec!["one"];
    assert!(push_unique_bounded(&mut values, "two", 2));
    assert!(!push_unique_bounded(&mut values, "two", 3));
    assert!(!push_unique_bounded(&mut values, "three", 2));
    assert_eq!(values, ["one", "two"]);
}
