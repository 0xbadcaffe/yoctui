use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeLayersOperation {
    ShowLayers,
    CreateLayer { directory: PathBuf, add: bool },
    AddLayers { directories: Vec<PathBuf> },
    RemoveLayers { directories: Vec<PathBuf> },
}

impl BitBakeLayersOperation {
    pub fn validate(&self) -> Result<(), BitBakeLayersOperationError> {
        match self {
            Self::ShowLayers => Ok(()),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitbake_layers_operations_validate_closed_path_sets() {
        BitBakeLayersOperation::CreateLayer {
            directory: "/layers/meta-demo".into(),
            add: true,
        }
        .validate()
        .unwrap();
        BitBakeLayersOperation::AddLayers {
            directories: vec!["/layers/meta-one".into(), "/layers/meta-two".into()],
        }
        .validate()
        .unwrap();
        assert!(
            BitBakeLayersOperation::RemoveLayers {
                directories: Vec::new()
            }
            .validate()
            .is_err()
        );
    }
}
