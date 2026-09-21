use super::*;

#[test]
fn menuconfig_children_always_receive_a_color_capable_terminal_identity() {
    for value in [None, Some(""), Some("dumb")] {
        let mut environment = BTreeMap::new();
        if let Some(value) = value {
            environment.insert("TERM".into(), value.into());
        }
        ensure_interactive_terminal_environment(&mut environment);
        assert_eq!(
            environment.get("TERM").map(String::as_str),
            Some("xterm-256color")
        );
    }

    let mut environment = BTreeMap::from([("TERM".into(), "screen-256color".into())]);
    ensure_interactive_terminal_environment(&mut environment);
    assert_eq!(
        environment.get("TERM").map(String::as_str),
        Some("screen-256color")
    );
}
