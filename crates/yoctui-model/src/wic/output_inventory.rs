#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicOutputKind {
    Wic,
    Direct,
    Bmap,
    Compressed,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WicOutputIdentity {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

impl WicOutputIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !absolute_normal_path(&self.path) {
            return Err("Wic output paths must be normalized and absolute");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicOutput {
    pub identity: WicOutputIdentity,
    pub kind: WicOutputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicOutputInventoryRequest {
    pub generation: u64,
    pub output_directory: PathBuf,
}

impl WicOutputInventoryRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 || !absolute_normal_path(&self.output_directory) {
            return Err("Wic output requests require a generation and normalized absolute root");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WicOutputInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: WicOutputInventoryRequest,
    },
    Available {
        request: WicOutputInventoryRequest,
        outputs: Vec<WicOutput>,
    },
    Partial {
        request: WicOutputInventoryRequest,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
    },
    Failed {
        request: WicOutputInventoryRequest,
        message: String,
    },
}
