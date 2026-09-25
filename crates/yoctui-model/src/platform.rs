use std::path::PathBuf;

use crate::{TerminalCreationKind, TerminalLaunchRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformComponent {
    Kernel,
    BootFirmware,
    UBoot,
    BiosUefi,
}

impl PlatformComponent {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kernel => "Kernel",
            Self::BootFirmware => "U-Boot / BIOS",
            Self::UBoot => "U-Boot",
            Self::BiosUefi => "BIOS / UEFI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlatformView {
    #[default]
    Configuration,
    DeviceTrees,
}

impl PlatformView {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Configuration => "Configuration",
            Self::DeviceTrees => "Device trees",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformFileKind {
    DotConfig,
    Dts,
    Dtsi,
    Dtb,
    Dtbo,
}

impl PlatformFileKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DotConfig => ".config",
            Self::Dts => "DTS",
            Self::Dtsi => "DTS include",
            Self::Dtb => "DTB",
            Self::Dtbo => "DT overlay",
        }
    }

    pub const fn is_text(self) -> bool {
        matches!(self, Self::DotConfig | Self::Dts | Self::Dtsi)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformFile {
    pub path: PathBuf,
    pub root: PathBuf,
    pub kind: PlatformFileKind,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DtcCompileOption {
    #[default]
    Symbols,
    Sort,
    Padding,
    ReserveEntries,
}

impl DtcCompileOption {
    pub const COUNT: usize = 4;

    pub const fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Sort,
            2 => Self::Padding,
            3 => Self::ReserveEntries,
            _ => Self::Symbols,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Symbols => "Generate symbols (-@)",
            Self::Sort => "Stable sort (-s)",
            Self::Padding => "Output padding (-p)",
            Self::ReserveEntries => "Reserve entries (-R)",
        }
    }
}

pub const DTC_PADDING_CHOICES: [u32; 5] = [0, 256, 1_024, 4_096, 16_384];
pub const DTC_RESERVE_CHOICES: [u32; 5] = [0, 1, 4, 8, 16];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcCompileDialog {
    pub component: PlatformComponent,
    pub source: PathBuf,
    pub output: PathBuf,
    pub root: PathBuf,
    pub program: PathBuf,
    pub selection: usize,
    pub symbols: bool,
    pub sort: bool,
    pub padding_bytes: u32,
    pub reserve_entries: u32,
}

impl DtcCompileDialog {
    pub fn new(component: PlatformComponent, file: &PlatformFile, program: PathBuf) -> Self {
        let stem = file
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("device-tree");
        Self {
            component,
            source: file.path.clone(),
            output: file.path.with_file_name(format!("{stem}.yoctui.dtb")),
            root: file.root.clone(),
            program,
            selection: 0,
            symbols: false,
            sort: false,
            padding_bytes: 0,
            reserve_entries: 0,
        }
    }

    pub const fn selected_option(&self) -> DtcCompileOption {
        DtcCompileOption::from_index(self.selection)
    }

    pub fn select(&mut self, delta: isize) {
        self.selection = if delta.is_negative() {
            self.selection.saturating_sub(delta.unsigned_abs())
        } else {
            self.selection
                .saturating_add(delta as usize)
                .min(DtcCompileOption::COUNT - 1)
        };
    }

    pub fn adjust(&mut self, delta: isize) {
        match self.selected_option() {
            DtcCompileOption::Symbols => self.symbols = !self.symbols,
            DtcCompileOption::Sort => self.sort = !self.sort,
            DtcCompileOption::Padding => {
                self.padding_bytes =
                    shifted_choice(self.padding_bytes, &DTC_PADDING_CHOICES, delta);
            }
            DtcCompileOption::ReserveEntries => {
                self.reserve_entries =
                    shifted_choice(self.reserve_entries, &DTC_RESERVE_CHOICES, delta);
            }
        }
    }

    pub fn option_value(&self, option: DtcCompileOption) -> String {
        match option {
            DtcCompileOption::Symbols => enabled_label(self.symbols).into(),
            DtcCompileOption::Sort => enabled_label(self.sort).into(),
            DtcCompileOption::Padding => format!("{} bytes", self.padding_bytes),
            DtcCompileOption::ReserveEntries => self.reserve_entries.to_string(),
        }
    }

    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = vec!["-I".into(), "dts".into(), "-O".into(), "dtb".into()];
        if self.symbols {
            arguments.push("-@".into());
        }
        if self.sort {
            arguments.push("-s".into());
        }
        if self.padding_bytes > 0 {
            arguments.extend(["-p".into(), self.padding_bytes.to_string()]);
        }
        if self.reserve_entries > 0 {
            arguments.extend(["-R".into(), self.reserve_entries.to_string()]);
        }
        arguments.extend([
            "-o".into(),
            self.output.display().to_string(),
            self.source.display().to_string(),
        ]);
        arguments
    }

    pub fn terminal_request(&self) -> TerminalLaunchRequest {
        TerminalLaunchRequest {
            name: format!(
                "compile {} device tree",
                self.component.label().to_ascii_lowercase()
            ),
            kind: TerminalCreationKind::Utility,
            cwd: self.root.clone(),
            program: self.program.clone(),
            arguments: self.arguments(),
        }
    }
}

const fn enabled_label(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

fn shifted_choice(current: u32, choices: &[u32], delta: isize) -> u32 {
    let index = choices
        .iter()
        .position(|choice| *choice == current)
        .unwrap_or(0);
    let next = if delta.is_negative() {
        index.saturating_sub(delta.unsigned_abs())
    } else {
        index.saturating_add(delta as usize).min(choices.len() - 1)
    };
    choices[next]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInventory {
    pub component: PlatformComponent,
    pub target: String,
    pub provider: Option<PathBuf>,
    pub tasks: Vec<String>,
    pub roots: Vec<PathBuf>,
    pub files: Vec<PlatformFile>,
    pub dtc: Option<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlatformInventoryState {
    #[default]
    NotLoaded,
    Loading,
    Available(PlatformInventory),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformWorkbench {
    pub view: PlatformView,
    pub inventory: PlatformInventoryState,
    pub config_selection: usize,
    pub device_tree_selection: usize,
    pub menuconfig_terminal: PlatformTerminalState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformTerminalState {
    pub name: Option<String>,
    pub prior_session_ids: Vec<u64>,
    pub session_id: Option<u64>,
    pub writer_control_requested: bool,
}

impl PlatformWorkbench {
    pub fn visible_files(&self) -> impl Iterator<Item = &PlatformFile> {
        let view = self.view;
        self.inventory().into_iter().flat_map(move |inventory| {
            inventory.files.iter().filter(move |file| match view {
                PlatformView::Configuration => file.kind == PlatformFileKind::DotConfig,
                PlatformView::DeviceTrees => file.kind != PlatformFileKind::DotConfig,
            })
        })
    }

    pub const fn inventory(&self) -> Option<&PlatformInventory> {
        match &self.inventory {
            PlatformInventoryState::Available(inventory) => Some(inventory),
            _ => None,
        }
    }

    pub fn selected_file(&self) -> Option<&PlatformFile> {
        let selection = match self.view {
            PlatformView::Configuration => self.config_selection,
            PlatformView::DeviceTrees => self.device_tree_selection,
        };
        self.visible_files().nth(selection)
    }

    pub fn select(&mut self, delta: isize) {
        let count = self.visible_files().count();
        let selection = match self.view {
            PlatformView::Configuration => &mut self.config_selection,
            PlatformView::DeviceTrees => &mut self.device_tree_selection,
        };
        *selection = if count == 0 {
            0
        } else if delta.is_negative() {
            selection.saturating_sub(delta.unsigned_abs())
        } else {
            selection.saturating_add(delta as usize).min(count - 1)
        };
    }

    pub fn cycle_view(&mut self) {
        self.view = match self.view {
            PlatformView::Configuration => PlatformView::DeviceTrees,
            PlatformView::DeviceTrees => PlatformView::Configuration,
        };
    }
}

#[cfg(test)]
#[path = "tests/platform/mod.rs"]
mod tests;
