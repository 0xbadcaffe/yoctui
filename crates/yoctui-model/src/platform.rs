use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInventory {
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
mod tests {
    use super::*;

    #[test]
    fn view_filters_and_selection_are_independent() {
        let mut state = PlatformWorkbench {
            inventory: PlatformInventoryState::Available(PlatformInventory {
                target: "virtual/kernel".into(),
                provider: None,
                tasks: vec![],
                roots: vec!["/work".into()],
                files: vec![
                    PlatformFile {
                        path: "/work/.config".into(),
                        root: "/work".into(),
                        kind: PlatformFileKind::DotConfig,
                        size_bytes: 1,
                    },
                    PlatformFile {
                        path: "/work/a.dts".into(),
                        root: "/work".into(),
                        kind: PlatformFileKind::Dts,
                        size_bytes: 2,
                    },
                    PlatformFile {
                        path: "/work/b.dtb".into(),
                        root: "/work".into(),
                        kind: PlatformFileKind::Dtb,
                        size_bytes: 3,
                    },
                ],
                dtc: None,
                limitations: vec![],
            }),
            ..PlatformWorkbench::default()
        };
        assert_eq!(state.visible_files().count(), 1);
        state.cycle_view();
        state.select(1);
        assert_eq!(
            state.selected_file().map(|file| file.kind),
            Some(PlatformFileKind::Dtb)
        );
        state.cycle_view();
        assert_eq!(
            state.selected_file().map(|file| file.kind),
            Some(PlatformFileKind::DotConfig)
        );
    }
}
