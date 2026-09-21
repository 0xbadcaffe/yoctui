use super::*;

#[test]
fn cache_policy_accepts_bitbake_booleans_and_keeps_missing_unknown() {
    for (value, expected) in [
        ("YES", Some(true)),
        ("false", Some(false)),
        ("", Some(false)),
        ("invalid", None),
    ] {
        assert_eq!(cache_policy_flag(value), expected);
    }
    let mut app = crate::App::new(16, 4096);
    assert!(app.cache_status_lines()[2].contains("Network: unknown"));
    app.workspace
        .variables
        .insert("BB_NO_NETWORK".into(), "false".into());
    app.workspace
        .variables
        .insert("BB_FETCH_PREMIRRORONLY".into(), "yes".into());
    assert!(app.cache_status_lines()[2].contains("premirrors only"));
    app.workspace
        .variables
        .insert("BB_FETCH_PREMIRRORONLY".into(), "0".into());
    assert!(app.cache_status_lines()[2].contains("Network: allowed"));
}
