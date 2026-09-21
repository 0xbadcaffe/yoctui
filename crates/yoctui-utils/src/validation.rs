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
#[path = "tests/validation/mod.rs"]
mod tests;
