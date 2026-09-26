use super::*;

pub const MAX_HARDWARE_DOCUMENTS: usize = 512;
pub const MAX_HARDWARE_BROWSER_ENTRIES: usize = 4096;
pub const MAX_HARDWARE_TEXT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_HARDWARE_RASTER_PIXELS: usize = 320 * 240;
pub const MAX_HARDWARE_QUERY_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareCategory {
    #[default]
    Board,
    Soc,
    Memory,
    Peripherals,
    Sensors,
    Other,
}

impl HardwareCategory {
    pub const ALL: [Self; 6] = [
        Self::Board,
        Self::Soc,
        Self::Memory,
        Self::Peripherals,
        Self::Sensors,
        Self::Other,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Board => "Board",
            Self::Soc => "SoC",
            Self::Memory => "Memory",
            Self::Peripherals => "Peripherals",
            Self::Sensors => "Sensors",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareDocumentKind {
    Pdf,
    Kicad,
    Svg,
    Raster,
}

impl HardwareDocumentKind {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "pdf" => Some(Self::Pdf),
            "kicad_sch" | "sch" => Some(Self::Kicad),
            "svg" => Some(Self::Svg),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp" => Some(Self::Raster),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Kicad => "KiCad schematic",
            Self::Svg => "SVG",
            Self::Raster => "Image",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareDocument {
    pub path: PathBuf,
    pub category: HardwareCategory,
    pub kind: HardwareDocumentKind,
}

impl HardwareDocument {
    pub fn validate(&self) -> Result<(), String> {
        if !self.path.is_absolute() {
            return Err("Hardware document paths must be absolute.".into());
        }
        if HardwareDocumentKind::from_path(&self.path) != Some(self.kind) {
            return Err("Hardware document extension does not match its stored kind.".into());
        }
        Ok(())
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

pub fn validate_hardware_documents(documents: &[HardwareDocument]) -> Result<(), String> {
    if documents.len() > MAX_HARDWARE_DOCUMENTS {
        return Err(format!(
            "Hardware library exceeds {MAX_HARDWARE_DOCUMENTS} documents."
        ));
    }
    let mut paths = BTreeSet::new();
    for document in documents {
        document.validate()?;
        if !paths.insert(document.path.clone()) {
            return Err(format!(
                "Hardware library contains duplicate path {}.",
                document.path.display()
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareBrowserEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    pub kind: Option<HardwareDocumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareBrowserState {
    pub directory: PathBuf,
    pub entries: Vec<HardwareBrowserEntry>,
    pub selection: usize,
    pub category: HardwareCategory,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareRgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareRaster {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<HardwareRgb>,
}

impl HardwareRaster {
    pub fn validate(&self) -> Result<(), String> {
        let expected = self
            .width
            .checked_mul(self.height)
            .ok_or_else(|| "Hardware raster dimensions overflow.".to_owned())?;
        if expected != self.pixels.len() || expected > MAX_HARDWARE_RASTER_PIXELS {
            return Err("Hardware raster exceeds its bounded dimensions.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwarePreview {
    Text {
        lines: Vec<String>,
        limitation: Option<String>,
    },
    Raster(HardwareRaster),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareViewerState {
    pub document: HardwareDocument,
    pub generation: u64,
    pub page: usize,
    pub page_count: usize,
    pub zoom_percent: u16,
    pub pan_x: usize,
    pub pan_y: usize,
    pub loading: bool,
    pub preview: Option<HardwarePreview>,
    pub searchable_text: Vec<String>,
    pub query: String,
    pub searching: bool,
    pub matches: Vec<usize>,
    pub match_selection: usize,
    pub error: Option<String>,
}

impl HardwareViewerState {
    pub(super) fn request(&self) -> HardwareLoadRequest {
        HardwareLoadRequest {
            generation: self.generation,
            document: self.document.clone(),
            page: self.page,
        }
    }

    pub(super) fn rebuild_matches(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = if query.is_empty() {
            Vec::new()
        } else {
            self.searchable_text
                .iter()
                .enumerate()
                .filter_map(|(line, value)| value.to_lowercase().contains(&query).then_some(line))
                .collect()
        };
        self.match_selection = self
            .match_selection
            .min(self.matches.len().saturating_sub(1));
        if let Some(line) = self.matches.get(self.match_selection) {
            self.pan_y = *line;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareLoadRequest {
    pub generation: u64,
    pub document: HardwareDocument,
    pub page: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HardwareState {
    pub documents: Vec<HardwareDocument>,
    pub missing_paths: BTreeSet<PathBuf>,
    pub category: HardwareCategory,
    pub selection: usize,
    pub browser: Option<HardwareBrowserState>,
    pub viewer: Option<HardwareViewerState>,
    pub removal_pending: Option<HardwareDocument>,
    pub last_directory: Option<PathBuf>,
}

impl HardwareState {
    pub fn visible_documents(&self) -> Vec<&HardwareDocument> {
        self.documents
            .iter()
            .filter(|document| document.category == self.category)
            .collect()
    }

    pub fn selected_document(&self) -> Option<&HardwareDocument> {
        self.visible_documents().get(self.selection).copied()
    }

    pub fn document_is_missing(&self, document: &HardwareDocument) -> bool {
        self.missing_paths.contains(&document.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareAction {
    Install(Vec<HardwareDocument>),
    SelectCategory {
        delta: isize,
    },
    SelectDocument {
        delta: isize,
    },
    OpenBrowser {
        directory: PathBuf,
    },
    BrowserLoaded {
        generation: u64,
        directory: PathBuf,
        entries: Vec<HardwareBrowserEntry>,
    },
    BrowserFailed {
        generation: u64,
        message: String,
    },
    SelectBrowserEntry {
        delta: isize,
    },
    SelectBrowserCategory {
        delta: isize,
    },
    EnterBrowserEntry,
    BrowseParent,
    ConfirmAdd,
    CancelBrowser,
    OpenSelected,
    Reload,
    PreviewLoaded {
        generation: u64,
        page_count: usize,
        preview: HardwarePreview,
        searchable_text: Vec<String>,
    },
    PreviewFailed {
        generation: u64,
        message: String,
    },
    ChangePage {
        delta: isize,
    },
    FirstPage,
    LastPage,
    Zoom {
        delta: i16,
    },
    ResetZoom,
    Pan {
        horizontal: isize,
        vertical: isize,
    },
    BeginSearch,
    AppendSearch(char),
    BackspaceSearch,
    FinishSearch,
    NextMatch {
        backwards: bool,
    },
    CloseViewer,
    BeginRemove,
    ConfirmRemove,
    CancelRemove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareEffect {
    Browse { generation: u64, directory: PathBuf },
    Load(HardwareLoadRequest),
    Persist(Vec<HardwareDocument>),
}

pub(super) fn next_generation(value: &mut u64) -> u64 {
    *value = value.wrapping_add(1).max(1);
    *value
}
