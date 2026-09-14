//! Devtool types.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishRequest {
    pub recipe: String,
    pub destination: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishPicker {
    pub identity: RecipeIdentity,
    pub layers: Vec<Layer>,
    pub selection: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolFinishPlan {
    pub identity: RecipeIdentity,
    pub layer: Layer,
}
impl DevtoolFinishPlan {
    pub fn request(&self) -> DevtoolFinishRequest {
        DevtoolFinishRequest {
            recipe: self.identity.name.clone(),
            destination: self.layer.path.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployRequest {
    pub recipe: String,
    pub target: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployDraft {
    pub identity: RecipeIdentity,
    pub target: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolDeployPlan {
    pub identity: RecipeIdentity,
    pub target: String,
}
impl DevtoolDeployPlan {
    pub fn request(&self) -> DevtoolDeployRequest {
        DevtoolDeployRequest {
            recipe: self.identity.name.clone(),
            target: self.target.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolResetPlan {
    pub identity: RecipeIdentity,
    pub source_path: PathBuf,
}
impl DevtoolResetPlan {
    pub fn operation(&self) -> DevtoolOperation {
        DevtoolOperation::Reset {
            recipe: self.identity.name.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevtoolOperation {
    Modify {
        recipe: String,
    },
    UpdateRecipe {
        recipe: String,
    },
    Finish {
        recipe: String,
        destination: PathBuf,
    },
    DeployTarget {
        recipe: String,
        target: String,
    },
    UndeployTarget {
        recipe: String,
        target: String,
    },
    Reset {
        recipe: String,
    },
    Upgrade {
        recipe: String,
    },
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DevtoolOperationError {
    #[error("Devtool recipe must be one non-option value without whitespace or control characters")]
    InvalidRecipe,
    #[error("Devtool target must be one non-option value without whitespace or control characters")]
    InvalidTarget,
    #[error("Devtool finish destination must be an absolute path")]
    RelativeFinishDestination,
}
impl DevtoolOperation {
    pub fn recipe(&self) -> &str {
        match self {
            Self::Modify { recipe }
            | Self::UpdateRecipe { recipe }
            | Self::Finish { recipe, .. }
            | Self::DeployTarget { recipe, .. }
            | Self::UndeployTarget { recipe, .. }
            | Self::Reset { recipe }
            | Self::Upgrade { recipe } => recipe,
        }
    }

    pub fn validate(&self) -> Result<(), DevtoolOperationError> {
        let valid_token = |value: &str| {
            !value.is_empty()
                && !value.starts_with('-')
                && value
                    .chars()
                    .all(|character| !character.is_whitespace() && !character.is_control())
        };
        if !valid_token(self.recipe()) {
            return Err(DevtoolOperationError::InvalidRecipe);
        }
        match self {
            Self::Finish { destination, .. } if !destination.is_absolute() => {
                Err(DevtoolOperationError::RelativeFinishDestination)
            }
            Self::DeployTarget { target, .. } | Self::UndeployTarget { target, .. }
                if !valid_token(target) =>
            {
                Err(DevtoolOperationError::InvalidTarget)
            }
            _ => Ok(()),
        }
    }
}
impl From<DevtoolFinishRequest> for DevtoolOperation {
    fn from(request: DevtoolFinishRequest) -> Self {
        Self::Finish {
            recipe: request.recipe,
            destination: request.destination,
        }
    }
}
impl From<DevtoolDeployRequest> for DevtoolOperation {
    fn from(request: DevtoolDeployRequest) -> Self {
        Self::DeployTarget {
            recipe: request.recipe,
            target: request.target,
        }
    }
}
