use std::path::{Component, Path};

pub(super) fn required<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} is required"));
    }
    plain(value, label)?;
    Ok(value)
}

pub(super) fn optional<'a>(value: &'a str, label: &str) -> Result<Option<&'a str>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    plain(value, label)?;
    Ok(Some(value))
}

pub(super) fn token<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = required(value, label)?;
    if value.starts_with('-') || value.chars().any(char::is_whitespace) {
        return Err(format!(
            "{label} must be one non-option value without whitespace"
        ));
    }
    Ok(value)
}

pub(super) fn optional_token<'a>(value: &'a str, label: &str) -> Result<Option<&'a str>, String> {
    optional(value, label)?
        .map(|value| token(value, label))
        .transpose()
}

pub(super) fn tokens(value: &str, label: &str, required: bool) -> Result<Vec<String>, String> {
    let values = value
        .split_whitespace()
        .map(|value| token(value, label).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    if required && values.is_empty() {
        return Err(format!("at least one {label} is required"));
    }
    if values.len() > 128 {
        return Err(format!("{label} accepts at most 128 values"));
    }
    Ok(values)
}

pub(super) fn absolute_path<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = required(value, label)?;
    let path = Path::new(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(format!("{label} must be a normalized absolute path"));
    }
    Ok(value)
}

pub(super) fn optional_absolute_path<'a>(
    value: &'a str,
    label: &str,
) -> Result<Option<&'a str>, String> {
    optional(value, label)?
        .map(|value| absolute_path(value, label))
        .transpose()
}

pub(super) fn port<'a>(value: &'a str, label: &str) -> Result<Option<&'a str>, String> {
    let Some(value) = optional_token(value, label)? else {
        return Ok(None);
    };
    if value.parse::<u16>().ok().filter(|port| *port > 0).is_none() {
        return Err(format!("{label} must be between 1 and 65535"));
    }
    Ok(Some(value))
}

pub(super) fn comma_list<'a>(value: &'a str, label: &str) -> Result<Option<&'a str>, String> {
    let Some(value) = optional(value, label)? else {
        return Ok(None);
    };
    if value.split(',').any(|item| {
        let item = item.trim();
        item.is_empty() || item.starts_with('-') || item.chars().any(char::is_whitespace)
    }) {
        return Err(format!(
            "{label} must be a comma-separated list without empty values"
        ));
    }
    Ok(Some(value))
}

pub(super) fn trailing_arguments(value: &str) -> Result<Vec<String>, String> {
    let values = value
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.len() > 64 {
        return Err("configure arguments accept at most 64 values".into());
    }
    if values
        .iter()
        .any(|value| value.chars().any(char::is_control))
    {
        return Err("configure arguments contain control characters".into());
    }
    Ok(values)
}

fn plain(value: &str, label: &str) -> Result<(), String> {
    if value.len() > 512 || value.chars().any(char::is_control) {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(())
}
