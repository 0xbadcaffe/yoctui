use super::*;

#[test]
fn authoritative_value_ignores_bitbake_diagnostics() {
    assert_eq!(
        authoritative_value("NOTE: reconnecting\npoky\n"),
        Some("poky".into())
    );
    assert_eq!(
        authoritative_value("\u{1b}[32mpoky\u{1b}[0m\n"),
        Some("poky".into())
    );
    assert_eq!(authoritative_value("NOTE: reconnecting\n"), None);
    assert_eq!(
        authoritative_token("poky\nThe variable 'DISTRO' is not defined"),
        Some("poky".into())
    );
}
