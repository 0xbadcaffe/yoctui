#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImageArtifactInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: ImageArtifactRequest,
    },
    AvailableEmpty {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
    },
    Available {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
    },
    Partial {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
        limitations: Vec<String>,
    },
    Failed {
        request: ImageArtifactRequest,
        message: String,
    },
}

impl ImageArtifactInventoryState {
    pub fn request(&self) -> Option<&ImageArtifactRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request, .. }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(request),
        }
    }

    pub fn inventory(&self) -> Option<&ImageArtifactInventory> {
        match self {
            Self::AvailableEmpty { inventory, .. }
            | Self::Available { inventory, .. }
            | Self::Partial { inventory, .. } => Some(inventory),
            Self::NotLoaded | Self::Loading { .. } | Self::Failed { .. } => None,
        }
    }

    pub fn artifacts(&self) -> Option<&[ImageArtifact]> {
        self.inventory()
            .map(|inventory| inventory.artifacts.as_slice())
    }
}
