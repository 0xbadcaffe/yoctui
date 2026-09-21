use super::*;

#[test]
fn cache_native_summary_parsing_is_checked() {
    let line = "NOTE: Sstate summary: Wanted 10 Local 3 Mirrors 2 Missed 5 Current 8 (50% match, 72% complete)";
    assert_eq!(
        parse_sstate_summary(line).unwrap().match_percent(),
        Some(50)
    );
    for line in [
        line.replace("Missed 5", "Missed 6"),
        line.replace("Local 3", "Local -3"),
        line.replace("Local 3", "Local 18446744073709551616"),
        format!("recipe: {line}"),
    ] {
        assert_eq!(parse_sstate_summary(&line), None);
    }
}
