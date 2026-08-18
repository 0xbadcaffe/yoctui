use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipetoolOperation {
    Create {
        source: String,
        outfile: PathBuf,
    },
    AppendFile {
        destination_layer: PathBuf,
        target_path: PathBuf,
        replacement_file: PathBuf,
    },
}

impl RecipetoolOperation {
    pub fn validate(&self) -> Result<(), RecipetoolOperationError> {
        match self {
            Self::Create { source, outfile } => {
                validate_token(source, RecipetoolOperationError::InvalidSource)?;
                validate_absolute(outfile, RecipetoolOperationError::InvalidOutput)?;
                if outfile.extension().and_then(|value| value.to_str()) != Some("bb") {
                    return Err(RecipetoolOperationError::InvalidOutput);
                }
            }
            Self::AppendFile {
                destination_layer,
                target_path,
                replacement_file,
            } => {
                validate_absolute(
                    destination_layer,
                    RecipetoolOperationError::InvalidDestinationLayer,
                )?;
                validate_absolute(target_path, RecipetoolOperationError::InvalidTargetPath)?;
                validate_absolute(
                    replacement_file,
                    RecipetoolOperationError::InvalidReplacementFile,
                )?;
            }
        }
        Ok(())
    }
}

fn validate_token(
    value: &str,
    error: RecipetoolOperationError,
) -> Result<(), RecipetoolOperationError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.chars().any(|character| character.is_control())
    {
        return Err(error);
    }
    Ok(())
}

fn validate_absolute(
    path: &std::path::Path,
    error: RecipetoolOperationError,
) -> Result<(), RecipetoolOperationError> {
    if !path.is_absolute() || path.parent().is_none() || path == std::path::Path::new("/") {
        return Err(error);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RecipetoolOperationError {
    #[error("Recipetool source must be a non-option token without control characters")]
    InvalidSource,
    #[error("Recipetool output must be an absolute .bb path")]
    InvalidOutput,
    #[error("Recipetool destination layer must be an absolute non-root path")]
    InvalidDestinationLayer,
    #[error("Recipetool target path must be an absolute non-root path")]
    InvalidTargetPath,
    #[error("Recipetool replacement file must be an absolute non-root path")]
    InvalidReplacementFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipetool_operations_validate_closed_create_and_appendfile_inputs() {
        RecipetoolOperation::Create {
            source: "https://example.invalid/demo.tar.gz".into(),
            outfile: "/layers/meta-demo/recipes-demo/demo.bb".into(),
        }
        .validate()
        .unwrap();
        RecipetoolOperation::AppendFile {
            destination_layer: "/layers/meta-demo".into(),
            target_path: "/etc/motd".into(),
            replacement_file: "/work/motd".into(),
        }
        .validate()
        .unwrap();
        assert!(
            RecipetoolOperation::Create {
                source: "--help".into(),
                outfile: "demo.bb".into(),
            }
            .validate()
            .is_err()
        );
    }
}
