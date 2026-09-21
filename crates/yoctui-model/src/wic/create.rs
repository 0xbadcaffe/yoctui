#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicCompression {
    None,
    Gzip,
    Bzip2,
    Xz,
}

impl WicCompression {
    pub fn argument(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Gzip => Some("gzip"),
            Self::Bzip2 => Some("bzip2"),
            Self::Xz => Some("xz"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateRequest {
    pub machine: String,
    pub image: String,
    pub kickstart: WicKickstartIdentity,
    pub output_directory: PathBuf,
    pub generate_bmap: bool,
    pub compression: WicCompression,
}

impl WicCreateRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !safe_name(&self.machine) || !safe_name(&self.image) {
            return Err("Wic machine and image identities must be bounded plain tokens");
        }
        self.kickstart.validate()?;
        if !absolute_normal_path(&self.output_directory) {
            return Err("Wic output directories must be normalized absolute paths");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateDraft {
    pub machine: String,
    pub image: String,
    pub kickstart: WicKickstartIdentity,
    pub output_directory: String,
    pub generate_bmap: bool,
    pub compression: WicCompression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WicCreateField {
    Machine,
    Image,
    Kickstart,
    OutputDirectory,
    GenerateBmap,
    Compression,
}

impl WicCreateField {
    const ALL: [Self; 6] = [
        Self::Machine,
        Self::Image,
        Self::Kickstart,
        Self::OutputDirectory,
        Self::GenerateBmap,
        Self::Compression,
    ];

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0);
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(Self::ALL.len() - 1)
        };
        Self::ALL[next]
    }

    pub fn is_read_only(self) -> bool {
        self == Self::Machine
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateDialog {
    pub draft: WicCreateDraft,
    pub selected_field: WicCreateField,
    pub editing: bool,
    pub validation_error: Option<String>,
}

impl WicCreateDialog {
    pub fn new(draft: WicCreateDraft) -> Self {
        Self {
            draft,
            selected_field: WicCreateField::Machine,
            editing: false,
            validation_error: None,
        }
    }

    pub fn selected_text_mut(&mut self) -> Option<(&mut String, usize)> {
        (self.selected_field == WicCreateField::OutputDirectory).then_some((
            &mut self.draft.output_directory,
            MAX_WIC_OUTPUT_DIRECTORY_INPUT_BYTES,
        ))
    }

    pub fn cycle_choice(&mut self, capability: &WicCapability, backwards: bool) -> bool {
        let WicCapability::Available {
            kickstarts,
            image_targets,
            ..
        } = capability
        else {
            return false;
        };
        match self.selected_field {
            WicCreateField::Image if !image_targets.is_empty() => {
                cycle_value(&mut self.draft.image, image_targets, backwards);
                true
            }
            WicCreateField::Kickstart if !kickstarts.is_empty() => {
                let values: Vec<_> = kickstarts
                    .iter()
                    .map(|kickstart| kickstart.identity.clone())
                    .collect();
                cycle_value(&mut self.draft.kickstart, &values, backwards);
                true
            }
            WicCreateField::GenerateBmap => {
                self.draft.generate_bmap = !self.draft.generate_bmap;
                true
            }
            WicCreateField::Compression => {
                self.draft.compression = match (self.draft.compression, backwards) {
                    (WicCompression::None, false) => WicCompression::Gzip,
                    (WicCompression::Gzip, false) => WicCompression::Bzip2,
                    (WicCompression::Bzip2, false) => WicCompression::Xz,
                    (WicCompression::Xz, false) => WicCompression::None,
                    (WicCompression::None, true) => WicCompression::Xz,
                    (WicCompression::Gzip, true) => WicCompression::None,
                    (WicCompression::Bzip2, true) => WicCompression::Gzip,
                    (WicCompression::Xz, true) => WicCompression::Bzip2,
                };
                true
            }
            _ => false,
        }
    }
}

fn cycle_value<T: Clone + PartialEq>(current: &mut T, values: &[T], backwards: bool) {
    let index = values
        .iter()
        .position(|value| value == current)
        .unwrap_or(0);
    let next = if backwards {
        index.checked_sub(1).unwrap_or(values.len() - 1)
    } else {
        (index + 1) % values.len()
    };
    *current = values[next].clone();
}

impl WicCreateDraft {
    pub fn preview(&self, capability: &WicCapability) -> Result<WicCreatePreview, &'static str> {
        if self.output_directory.trim() != self.output_directory
            || self.output_directory.chars().any(char::is_control)
        {
            return Err("Wic output directories must not contain surrounding space or controls");
        }
        let request = WicCreateRequest {
            machine: self.machine.clone(),
            image: self.image.clone(),
            kickstart: self.kickstart.clone(),
            output_directory: PathBuf::from(&self.output_directory),
            generate_bmap: self.generate_bmap,
            compression: self.compression,
        };
        request.validate()?;
        let (executable, kickstart) = capability.resolve(&request.kickstart, &request.image)?;
        let mut argv = vec![
            executable.to_path_buf(),
            "create".into(),
            request.kickstart.argument(),
            "-e".into(),
            request.image.clone().into(),
            "-o".into(),
            request.output_directory.clone(),
        ];
        if request.generate_bmap {
            argv.push("--bmap".into());
        }
        if let Some(compression) = request.compression.argument() {
            argv.extend(["--compress-with".into(), compression.into()]);
        }
        Ok(WicCreatePreview {
            request,
            kickstart: kickstart.clone(),
            argv,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreatePreview {
    pub request: WicCreateRequest,
    pub kickstart: WicKickstart,
    pub argv: Vec<PathBuf>,
}
