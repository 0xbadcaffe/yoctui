#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RootfsCompositionState {
    #[default]
    NotLoaded,
    Loading {
        request: RootfsCompositionRequest,
    },
    AvailableEmpty {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
    },
    Available {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
    },
    Partial {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
        limitations: Vec<String>,
    },
    Unavailable {
        request: RootfsCompositionRequest,
        reason: String,
    },
    Failed {
        request: RootfsCompositionRequest,
        message: String,
    },
}

impl RootfsCompositionState {
    pub fn request(&self) -> Option<&RootfsCompositionRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request, .. }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Unavailable { request, .. }
            | Self::Failed { request, .. } => Some(request),
        }
    }

    pub fn composition(&self) -> Option<&RootfsComposition> {
        match self {
            Self::AvailableEmpty { composition, .. }
            | Self::Available { composition, .. }
            | Self::Partial { composition, .. } => Some(composition),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::Unavailable { .. }
            | Self::Failed { .. } => None,
        }
    }
}

fn rootfs_text_is_valid(value: &str) -> bool {
    value.len() <= MAX_ROOTFS_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn rootfs_path_is_valid(path: &Path) -> bool {
    path.is_absolute()
        && path.as_os_str().as_encoded_bytes().len() <= MAX_ROOTFS_PATH_BYTES
        && !path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}
