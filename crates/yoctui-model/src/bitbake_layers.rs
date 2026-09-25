use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeLayersOperation {
    ShowLayers,
    ShowRecipes { pattern: Option<String> },
    ShowOverlayed,
    CreateLayer { directory: PathBuf, add: bool },
    AddLayers { directories: Vec<PathBuf> },
    RemoveLayers { directories: Vec<PathBuf> },
}

impl BitBakeLayersOperation {
    pub fn validate(&self) -> Result<(), BitBakeLayersOperationError> {
        match self {
            Self::ShowLayers | Self::ShowOverlayed => Ok(()),
            Self::ShowRecipes { pattern } => {
                if let Some(pattern) = pattern
                    && (pattern.is_empty()
                        || pattern.len() > 256
                        || pattern.chars().any(char::is_control))
                {
                    return Err(BitBakeLayersOperationError::InvalidRecipePattern);
                }
                Ok(())
            }
            Self::CreateLayer { directory, .. } => validate_directory(directory),
            Self::AddLayers { directories } | Self::RemoveLayers { directories } => {
                if directories.is_empty() || directories.len() > 64 {
                    return Err(BitBakeLayersOperationError::InvalidDirectoryCount);
                }
                for directory in directories {
                    validate_directory(directory)?;
                }
                Ok(())
            }
        }
    }
}

fn validate_directory(directory: &std::path::Path) -> Result<(), BitBakeLayersOperationError> {
    if !directory.is_absolute()
        || directory == std::path::Path::new("/")
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeLayersOperationError {
    #[error("layer directory must be an absolute normalized non-root path")]
    InvalidDirectory,
    #[error("layer operation requires between 1 and 64 directories")]
    InvalidDirectoryCount,
    #[error("recipe pattern must contain 1 to 256 printable bytes")]
    InvalidRecipePattern,
}

#[cfg(test)]
#[path = "tests/bitbake_layers/mod.rs"]
mod tests;
