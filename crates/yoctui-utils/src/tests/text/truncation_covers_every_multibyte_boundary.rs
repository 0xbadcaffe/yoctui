use super::*;

#[test]
fn truncation_covers_every_multibyte_boundary() {
    for (limit, expected) in [
        (0, ""),
        (1, "a"),
        (2, "a"),
        (3, "aλ"),
        (4, "aλ"),
        (7, "aλ🙂"),
        (99, "aλ🙂z"),
    ] {
        let mut value = "aλ🙂z".to_owned();
        truncate_utf8(&mut value, limit);
        assert_eq!(value, expected);
    }
}
