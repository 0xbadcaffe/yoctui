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
    fn request(&self) -> HardwareLoadRequest {
        HardwareLoadRequest {
            generation: self.generation,
            document: self.document.clone(),
            page: self.page,
        }
    }

    fn rebuild_matches(&mut self) {
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

fn next_generation(value: &mut u64) -> u64 {
    *value = value.wrapping_add(1).max(1);
    *value
}

pub(crate) fn reduce_hardware(app: &mut App, action: HardwareAction) -> Option<Effect> {
    let state = &mut app.hardware;
    match action {
        HardwareAction::Install(documents) => match validate_hardware_documents(&documents) {
            Ok(()) => {
                state.documents = documents;
                state.selection = 0;
            }
            Err(message) => app.notification = Some(message),
        },
        HardwareAction::SelectCategory { delta } => {
            let index = HardwareCategory::ALL
                .iter()
                .position(|category| category == &state.category)
                .unwrap_or(0);
            state.category =
                HardwareCategory::ALL[shifted_index(index, delta, HardwareCategory::ALL.len())];
            state.selection = 0;
        }
        HardwareAction::SelectDocument { delta } => {
            state.selection =
                shifted_index(state.selection, delta, state.visible_documents().len());
        }
        HardwareAction::OpenBrowser { directory } => {
            let generation = state
                .browser
                .as_ref()
                .map_or(1, |browser| browser.generation.wrapping_add(1).max(1));
            state.browser = Some(HardwareBrowserState {
                directory: directory.clone(),
                entries: Vec::new(),
                selection: 0,
                category: state.category,
                generation,
                loading: true,
                error: None,
            });
            return Some(Effect::Hardware(HardwareEffect::Browse {
                generation,
                directory,
            }));
        }
        HardwareAction::BrowserLoaded {
            generation,
            directory,
            mut entries,
        } => {
            if let Some(browser) = &mut state.browser
                && browser.generation == generation
            {
                entries.truncate(MAX_HARDWARE_BROWSER_ENTRIES);
                browser.directory = directory;
                browser.entries = entries;
                browser.selection = 0;
                browser.loading = false;
                browser.error = None;
            }
        }
        HardwareAction::BrowserFailed {
            generation,
            message,
        } => {
            if let Some(browser) = &mut state.browser
                && browser.generation == generation
            {
                browser.loading = false;
                browser.error = Some(message);
            }
        }
        HardwareAction::SelectBrowserEntry { delta } => {
            if let Some(browser) = &mut state.browser {
                browser.selection = shifted_index(browser.selection, delta, browser.entries.len());
            }
        }
        HardwareAction::SelectBrowserCategory { delta } => {
            if let Some(browser) = &mut state.browser {
                let index = HardwareCategory::ALL
                    .iter()
                    .position(|value| value == &browser.category)
                    .unwrap_or(0);
                browser.category =
                    HardwareCategory::ALL[shifted_index(index, delta, HardwareCategory::ALL.len())];
            }
        }
        HardwareAction::EnterBrowserEntry => {
            let browser = state.browser.as_mut()?;
            let entry = browser.entries.get(browser.selection)?.clone();
            if entry.is_directory {
                browser.generation = next_generation(&mut browser.generation);
                browser.loading = true;
                browser.error = None;
                return Some(Effect::Hardware(HardwareEffect::Browse {
                    generation: browser.generation,
                    directory: entry.path,
                }));
            }
        }
        HardwareAction::BrowseParent => {
            let browser = state.browser.as_mut()?;
            let parent = browser.directory.parent()?.to_path_buf();
            browser.generation = next_generation(&mut browser.generation);
            browser.loading = true;
            browser.error = None;
            return Some(Effect::Hardware(HardwareEffect::Browse {
                generation: browser.generation,
                directory: parent,
            }));
        }
        HardwareAction::ConfirmAdd => {
            let browser = state.browser.as_ref()?;
            let entry = browser.entries.get(browser.selection)?;
            let Some(kind) = entry.kind else { return None };
            if entry.is_directory
                || state
                    .documents
                    .iter()
                    .any(|document| document.path == entry.path)
            {
                app.notification = Some("That Hardware document is already in the library.".into());
                return None;
            }
            let document = HardwareDocument {
                path: entry.path.clone(),
                category: browser.category,
                kind,
            };
            if let Err(message) = document.validate() {
                app.notification = Some(message);
                return None;
            }
            if state.documents.len() >= MAX_HARDWARE_DOCUMENTS {
                app.notification = Some(format!(
                    "Hardware library is limited to {MAX_HARDWARE_DOCUMENTS} documents."
                ));
                return None;
            }
            state.last_directory = Some(browser.directory.clone());
            state.category = browser.category;
            state.documents.push(document);
            state.documents.sort_by_key(|document| {
                (
                    HardwareCategory::ALL
                        .iter()
                        .position(|value| value == &document.category),
                    document.name().to_lowercase(),
                )
            });
            state.selection = state.visible_documents().len().saturating_sub(1);
            state.browser = None;
            return Some(Effect::Hardware(HardwareEffect::Persist(
                state.documents.clone(),
            )));
        }
        HardwareAction::CancelBrowser => state.browser = None,
        HardwareAction::OpenSelected => {
            let document = state.selected_document()?.clone();
            let generation = state
                .viewer
                .as_ref()
                .map_or(1, |viewer| viewer.generation.wrapping_add(1).max(1));
            let viewer = HardwareViewerState {
                document,
                generation,
                page: 1,
                page_count: 1,
                zoom_percent: 100,
                pan_x: 0,
                pan_y: 0,
                loading: true,
                preview: None,
                searchable_text: Vec::new(),
                query: String::new(),
                searching: false,
                matches: Vec::new(),
                match_selection: 0,
                error: None,
            };
            let request = viewer.request();
            state.viewer = Some(viewer);
            return Some(Effect::Hardware(HardwareEffect::Load(request)));
        }
        HardwareAction::Reload => {
            let viewer = state.viewer.as_mut()?;
            viewer.generation = next_generation(&mut viewer.generation);
            viewer.loading = true;
            viewer.error = None;
            return Some(Effect::Hardware(HardwareEffect::Load(viewer.request())));
        }
        HardwareAction::PreviewLoaded {
            generation,
            page_count,
            preview,
            mut searchable_text,
        } => {
            let viewer = state.viewer.as_mut()?;
            if viewer.generation != generation {
                return None;
            }
            if let HardwarePreview::Raster(raster) = &preview
                && let Err(message) = raster.validate()
            {
                viewer.loading = false;
                viewer.error = Some(message);
                return None;
            }
            let mut bytes = 0usize;
            searchable_text.retain(|line| {
                bytes = bytes.saturating_add(line.len());
                bytes <= MAX_HARDWARE_TEXT_BYTES
            });
            viewer.page_count = page_count.max(1);
            viewer.page = viewer.page.min(viewer.page_count).max(1);
            viewer.preview = Some(preview);
            viewer.searchable_text = searchable_text;
            viewer.loading = false;
            viewer.error = None;
            viewer.rebuild_matches();
        }
        HardwareAction::PreviewFailed {
            generation,
            message,
        } => {
            let viewer = state.viewer.as_mut()?;
            if viewer.generation == generation {
                viewer.loading = false;
                viewer.error = Some(message);
            }
        }
        HardwareAction::ChangePage { delta } => {
            let viewer = state.viewer.as_mut()?;
            let page = shifted_index(viewer.page.saturating_sub(1), delta, viewer.page_count) + 1;
            if page != viewer.page {
                viewer.page = page;
                viewer.generation = next_generation(&mut viewer.generation);
                viewer.loading = true;
                viewer.pan_x = 0;
                viewer.pan_y = 0;
                return Some(Effect::Hardware(HardwareEffect::Load(viewer.request())));
            }
        }
        HardwareAction::FirstPage => {
            return reduce_hardware(app, HardwareAction::ChangePage { delta: isize::MIN });
        }
        HardwareAction::LastPage => {
            return reduce_hardware(app, HardwareAction::ChangePage { delta: isize::MAX });
        }
        HardwareAction::Zoom { delta } => {
            let viewer = state.viewer.as_mut()?;
            viewer.zoom_percent = viewer
                .zoom_percent
                .saturating_add_signed(delta)
                .clamp(25, 400);
        }
        HardwareAction::ResetZoom => {
            let viewer = state.viewer.as_mut()?;
            viewer.zoom_percent = 100;
            viewer.pan_x = 0;
            viewer.pan_y = 0;
        }
        HardwareAction::Pan {
            horizontal,
            vertical,
        } => {
            let viewer = state.viewer.as_mut()?;
            viewer.pan_x = viewer.pan_x.saturating_add_signed(horizontal);
            viewer.pan_y = viewer.pan_y.saturating_add_signed(vertical);
        }
        HardwareAction::BeginSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.searching = true;
            }
        }
        HardwareAction::AppendSearch(character) => {
            if let Some(viewer) = &mut state.viewer
                && viewer.query.chars().count() < MAX_HARDWARE_QUERY_CHARS
            {
                viewer.query.push(character);
                viewer.rebuild_matches();
            }
        }
        HardwareAction::BackspaceSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.query.pop();
                viewer.rebuild_matches();
            }
        }
        HardwareAction::FinishSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.searching = false;
            }
        }
        HardwareAction::NextMatch { backwards } => {
            if let Some(viewer) = &mut state.viewer
                && !viewer.matches.is_empty()
            {
                viewer.match_selection = if backwards {
                    viewer
                        .match_selection
                        .checked_sub(1)
                        .unwrap_or(viewer.matches.len() - 1)
                } else {
                    (viewer.match_selection + 1) % viewer.matches.len()
                };
                viewer.pan_y = viewer.matches[viewer.match_selection];
            }
        }
        HardwareAction::CloseViewer => state.viewer = None,
        HardwareAction::BeginRemove => state.removal_pending = state.selected_document().cloned(),
        HardwareAction::ConfirmRemove => {
            let document = state.removal_pending.take()?;
            state
                .documents
                .retain(|candidate| candidate.path != document.path);
            state.selection = state
                .selection
                .min(state.visible_documents().len().saturating_sub(1));
            return Some(Effect::Hardware(HardwareEffect::Persist(
                state.documents.clone(),
            )));
        }
        HardwareAction::CancelRemove => state.removal_pending = None,
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(name: &str, category: HardwareCategory) -> HardwareDocument {
        HardwareDocument {
            path: PathBuf::from(format!("/tmp/{name}.pdf")),
            category,
            kind: HardwareDocumentKind::Pdf,
        }
    }

    #[test]
    fn hardware_catalog_categories_selection_and_persistence_are_bounded() {
        let mut app = App::new(100, 100_000);
        let board = document("board", HardwareCategory::Board);
        let sensor = document("sensor", HardwareCategory::Sensors);
        update(
            &mut app,
            Action::Hardware(HardwareAction::Install(vec![board.clone(), sensor.clone()])),
        );
        assert_eq!(app.hardware.selected_document(), Some(&board));
        update(
            &mut app,
            Action::Hardware(HardwareAction::SelectCategory { delta: 4 }),
        );
        assert_eq!(app.hardware.selected_document(), Some(&sensor));
        update(&mut app, Action::Hardware(HardwareAction::BeginRemove));
        assert_eq!(
            update(&mut app, Action::Hardware(HardwareAction::ConfirmRemove)),
            Some(Effect::Hardware(HardwareEffect::Persist(vec![board])))
        );
    }

    #[test]
    fn hardware_viewer_rejects_stale_and_invalid_raster_results() {
        let mut app = App::new(100, 100_000);
        update(
            &mut app,
            Action::Hardware(HardwareAction::Install(vec![document(
                "board",
                HardwareCategory::Board,
            )])),
        );
        let effect = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
        assert!(matches!(
            effect,
            Some(Effect::Hardware(HardwareEffect::Load(_)))
        ));
        update(
            &mut app,
            Action::Hardware(HardwareAction::PreviewLoaded {
                generation: 99,
                page_count: 1,
                preview: HardwarePreview::Text {
                    lines: vec!["stale".into()],
                    limitation: None,
                },
                searchable_text: vec![],
            }),
        );
        assert!(app.hardware.viewer.as_ref().unwrap().loading);
        update(
            &mut app,
            Action::Hardware(HardwareAction::PreviewLoaded {
                generation: 1,
                page_count: 1,
                preview: HardwarePreview::Raster(HardwareRaster {
                    width: 2,
                    height: 2,
                    pixels: vec![],
                }),
                searchable_text: vec![],
            }),
        );
        assert!(app.hardware.viewer.as_ref().unwrap().error.is_some());
    }

    #[test]
    fn hardware_search_zoom_page_and_pan_transitions_are_pure() {
        let mut app = App::new(100, 100_000);
        update(
            &mut app,
            Action::Hardware(HardwareAction::Install(vec![document(
                "board",
                HardwareCategory::Board,
            )])),
        );
        update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
        update(
            &mut app,
            Action::Hardware(HardwareAction::PreviewLoaded {
                generation: 1,
                page_count: 3,
                preview: HardwarePreview::Text {
                    lines: vec!["alpha".into(), "beta alpha".into()],
                    limitation: None,
                },
                searchable_text: vec!["alpha".into(), "beta alpha".into()],
            }),
        );
        update(&mut app, Action::Hardware(HardwareAction::BeginSearch));
        for value in "alpha".chars() {
            update(
                &mut app,
                Action::Hardware(HardwareAction::AppendSearch(value)),
            );
        }
        update(
            &mut app,
            Action::Hardware(HardwareAction::NextMatch { backwards: false }),
        );
        update(
            &mut app,
            Action::Hardware(HardwareAction::Zoom { delta: 50 }),
        );
        update(
            &mut app,
            Action::Hardware(HardwareAction::Pan {
                horizontal: 3,
                vertical: 2,
            }),
        );
        let viewer = app.hardware.viewer.as_ref().unwrap();
        assert_eq!(
            (
                viewer.match_selection,
                viewer.pan_x,
                viewer.pan_y,
                viewer.zoom_percent
            ),
            (1, 3, 3, 150)
        );
        assert!(matches!(
            update(&mut app, Action::Hardware(HardwareAction::LastPage)),
            Some(Effect::Hardware(HardwareEffect::Load(
                HardwareLoadRequest { page: 3, .. }
            )))
        ));
    }
}
