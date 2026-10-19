use std::path::Path;
use yoctui_utils::*;

#[test]
fn normal_paths_reject_interior_current_directory_components() {
    assert!(!is_absolute_normal_path(Path::new("/tmp/./build")));
    assert!(!is_absolute_normal_path(Path::new("/tmp/build/.")));
}

#[cfg(unix)]
#[test]
fn normal_paths_preserve_non_utf8_names_while_rejecting_dot_components() {
    use std::os::unix::ffi::OsStrExt;
    let valid = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff/build"));
    let invalid = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff/./build"));
    assert!(is_absolute_normal_path(valid));
    assert!(!is_absolute_normal_path(invalid));
    assert!(is_absolute_normal_path_within(
        valid,
        valid.as_os_str().len()
    ));
    assert!(!is_absolute_normal_path_within(
        valid,
        valid.as_os_str().len() - 1
    ));
}

#[test]
fn normal_paths_reject_traversal_in_any_component() {
    for path in [
        "/../image",
        "/tmp/../image",
        "/tmp/image/..",
        "/tmp/../../image",
    ] {
        assert!(!is_absolute_normal_path(Path::new(path)), "{path}");
    }
}

#[test]
fn normal_paths_reject_empty_relative_and_root_paths() {
    for path in ["", ".", "..", "build", "./build", "/", "//"] {
        assert!(!is_absolute_normal_path(Path::new(path)), "{path}");
    }
}

#[test]
fn normal_paths_allow_unicode_spaces_hidden_names_and_dot_substrings() {
    for path in [
        "/tmp/茶 image",
        "/tmp/.config",
        "/tmp/a..b",
        "/tmp/.../build",
    ] {
        assert!(is_absolute_normal_path(Path::new(path)), "{path}");
    }
}

#[test]
fn path_budget_counts_encoded_bytes_not_characters() {
    let path = Path::new("/茶");
    assert!(!is_absolute_normal_path_within(path, 3));
    assert!(is_absolute_normal_path_within(path, 4));
    assert!(is_absolute_normal_path_within(path, usize::MAX));
}

#[test]
fn path_budget_rejects_valid_paths_when_zero() {
    assert!(!is_absolute_normal_path_within(Path::new("/tmp"), 0));
}

#[test]
fn plain_text_rejects_every_ascii_control_character() {
    for byte in (0..=31).chain([127]) {
        assert!(!is_bounded_plain_text(
            &format!("a{}b", char::from(byte)),
            32
        ));
    }
}

#[test]
fn plain_text_rejects_unicode_control_characters() {
    assert!(!is_bounded_plain_text("a\u{85}b", 16));
    assert!(!is_bounded_plain_text("a\u{9f}b", 16));
}

#[test]
fn plain_text_accepts_whitespace_that_is_not_a_control() {
    assert!(is_bounded_plain_text("ordinary space", 14));
    assert!(is_bounded_plain_text("a\u{a0}b", 4));
}

#[test]
fn plain_text_budget_is_in_bytes_and_excludes_empty_values() {
    assert!(!is_bounded_plain_text("", usize::MAX));
    assert!(!is_bounded_plain_text("茶", 2));
    assert!(is_bounded_plain_text("茶", 3));
}

#[test]
fn identifiers_accept_the_complete_documented_ascii_alphabet() {
    assert!(is_bounded_identifier("aZ09-_.+", 8));
    assert!(is_bounded_identifier(
        "core-image-minimal+ptest.qemux86_64",
        128
    ));
}

#[test]
fn identifiers_reject_shell_metacharacters_and_path_separators() {
    for suffix in [
        '/', '\\', ' ', '\n', ';', '$', '`', '|', '&', '(', ')', ':', '=',
    ] {
        assert!(!is_bounded_identifier(&format!("recipe{suffix}"), 128));
    }
}

#[test]
fn identifiers_reject_unicode_and_dot_only_navigation_names() {
    for value in ["", ".", "..", "café", "茶", "🙂"] {
        assert!(!is_bounded_identifier(value, 128), "{value}");
    }
}

#[test]
fn identifiers_enforce_exact_byte_limits() {
    assert!(!is_bounded_identifier("a", 0));
    assert!(is_bounded_identifier("a", 1));
    assert!(is_bounded_identifier("abc", 3));
    assert!(!is_bounded_identifier("abcd", 3));
}

#[test]
fn unique_push_with_zero_capacity_never_mutates() {
    let mut values = Vec::new();
    assert!(!push_unique_bounded(&mut values, "a", 0));
    assert!(values.is_empty());
}

#[test]
fn unique_push_preserves_order_and_rejects_duplicates_and_overflow() {
    let mut values = vec!["a"];
    assert!(!push_unique_bounded(&mut values, "a", 3));
    assert!(push_unique_bounded(&mut values, "b", 3));
    assert!(push_unique_bounded(&mut values, "c", 3));
    assert!(!push_unique_bounded(&mut values, "d", 3));
    assert_eq!(values, ["a", "b", "c"]);
}

#[test]
fn unique_push_does_not_trim_preexisting_over_capacity_values() {
    let mut values = vec![1, 2, 3];
    assert!(!push_unique_bounded(&mut values, 4, 1));
    assert_eq!(values, [1, 2, 3]);
}
