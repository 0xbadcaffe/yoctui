use std::path::{Path, PathBuf};

use thiserror::Error;

pub(super) fn validate_directories(
    directories: &[PathBuf],
) -> Result<(), BitBakeLayersOperationError> {
    if directories.is_empty() || directories.len() > 64 {
        return Err(BitBakeLayersOperationError::InvalidCount {
            field: "directories",
        });
    }
    directories
        .iter()
        .try_for_each(|directory| validate_directory(directory))
}

pub(super) fn validate_directory(directory: &Path) -> Result<(), BitBakeLayersOperationError> {
    if directory.to_str().is_none()
        || !directory.is_absolute()
        || directory == Path::new("/")
        || directory.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(BitBakeLayersOperationError::InvalidDirectory);
    }
    Ok(())
}

pub(super) fn validate_values(
    values: &[String],
    field: &'static str,
    required: bool,
) -> Result<(), BitBakeLayersOperationError> {
    if values.len() > 64 || (required && values.is_empty()) {
        return Err(BitBakeLayersOperationError::InvalidCount { field });
    }
    values
        .iter()
        .try_for_each(|value| validate_value(value, field))
}

pub(super) fn validate_optional(
    value: &Option<String>,
    field: &'static str,
) -> Result<(), BitBakeLayersOperationError> {
    value
        .as_ref()
        .map_or(Ok(()), |value| validate_value(value, field))
}

pub(super) fn validate_optional_csv(
    value: &Option<String>,
    field: &'static str,
) -> Result<(), BitBakeLayersOperationError> {
    if let Some(value) = value {
        validate_value(value, field)?;
        if value.split(',').any(str::is_empty) {
            return Err(BitBakeLayersOperationError::InvalidValue { field });
        }
    }
    Ok(())
}

pub(super) fn validate_value(
    value: &str,
    field: &'static str,
) -> Result<(), BitBakeLayersOperationError> {
    if value.is_empty()
        || value.len() > 512
        || value.starts_with('-')
        || value.chars().any(char::is_control)
    {
        return Err(BitBakeLayersOperationError::InvalidValue { field });
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeLayersOperationError {
    #[error("layer directory must be an absolute normalized non-root path")]
    InvalidDirectory,
    #[error("{field} require between 1 and 64 values")]
    InvalidCount { field: &'static str },
    #[error("{field} contain an unsupported value")]
    InvalidValue { field: &'static str },
    #[error("layer priority must be an unsigned integer")]
    InvalidPriority,
    #[error("custom references must use REPOSITORY:REFERENCE")]
    InvalidCustomReference,
}
