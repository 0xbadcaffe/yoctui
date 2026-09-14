use std::path::{Component, Path};

/// Validate a non-root absolute path whose components never traverse upward or
/// redundantly refer to the current directory.
pub fn is_absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path.parent().is_some()
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

pub fn is_absolute_normal_path_within(path: &Path, maximum_bytes: usize) -> bool {
    path.as_os_str().as_encoded_bytes().len() <= maximum_bytes && is_absolute_normal_path(path)
}

pub fn is_bounded_plain_text(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= maximum_bytes && !value.chars().any(char::is_control)
}

/// Validate the shared BitBake identifier alphabet used by recipe, machine,
/// distro, image and test selector identities.
pub fn is_bounded_identifier(value: &str, maximum_bytes: usize) -> bool {
    !matches!(value, "" | "." | "..")
        && value.len() <= maximum_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+'))
}

pub fn push_unique_bounded<T: PartialEq>(values: &mut Vec<T>, value: T, maximum: usize) -> bool {
    if values.len() >= maximum || values.contains(&value) {
        return false;
    }
    values.push(value);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_paths_are_absolute_non_root_and_traversal_free() {
        let separator = std::path::MAIN_SEPARATOR;
        let valid = std::path::PathBuf::from(format!("{separator}tmp{separator}image.wic"));
        assert!(is_absolute_normal_path(&valid));
        assert!(is_absolute_normal_path_within(
            &valid,
            valid.as_os_str().len()
        ));
        assert!(!is_absolute_normal_path(Path::new("relative/image.wic")));
        assert!(!is_absolute_normal_path(Path::new(
            std::path::MAIN_SEPARATOR_STR
        )));
        assert!(!is_absolute_normal_path(Path::new(&format!(
            "{separator}tmp{separator}..{separator}image.wic"
        ))));
        assert!(!is_absolute_normal_path_within(&valid, 1));
    }

    #[test]
    fn shared_text_and_identifier_rules_cover_boundaries() {
        assert!(is_bounded_plain_text("image", 5));
        assert!(!is_bounded_plain_text("", 5));
        assert!(!is_bounded_plain_text("bad\n", 5));
        assert!(is_bounded_identifier("core-image_minimal+dev", 256));
        for invalid in ["", ".", "..", "has space", "λ"] {
            assert!(!is_bounded_identifier(invalid, 256));
        }
    }

    #[test]
    fn bounded_unique_push_reports_insertions() {
        let mut values = vec!["one"];
        assert!(push_unique_bounded(&mut values, "two", 2));
        assert!(!push_unique_bounded(&mut values, "two", 3));
        assert!(!push_unique_bounded(&mut values, "three", 2));
        assert_eq!(values, ["one", "two"]);
    }
}
