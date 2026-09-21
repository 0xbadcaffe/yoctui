#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WicDeviceIdentity {
    pub path: PathBuf,
    pub major_minor: String,
    pub size_bytes: u64,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub transport: Option<String>,
}

impl WicDeviceIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !absolute_normal_path(&self.path)
            || !self.path.starts_with("/dev")
            || self.path.as_os_str().len() > 4_096
            || self.major_minor.is_empty()
            || self.major_minor.len() > 32
            || !self
                .major_minor
                .chars()
                .all(|character| character.is_ascii_digit() || character == ':')
        {
            return Err(
                "Wic device identities must use an exact normalized /dev path and major:minor",
            );
        }
        let valid_major_minor = self
            .major_minor
            .split_once(':')
            .is_some_and(|(major, minor)| {
                !major.is_empty()
                    && !minor.is_empty()
                    && major.chars().all(|character| character.is_ascii_digit())
                    && minor.chars().all(|character| character.is_ascii_digit())
            });
        if !valid_major_minor
            || self
                .model
                .iter()
                .chain(self.serial.iter())
                .chain(self.transport.iter())
                .any(|value| value.len() > 256 || value.chars().any(char::is_control))
        {
            return Err("Wic device metadata is malformed or exceeds its safety bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDevice {
    pub identity: WicDeviceIdentity,
    pub removable: bool,
    pub writable: bool,
    pub read_only: bool,
    pub descendant_mounts: Vec<PathBuf>,
    pub unavailable_reason: Option<String>,
}

impl WicDevice {
    pub fn eligible_for(&self, image: &WicOutputIdentity) -> Result<(), &'static str> {
        self.identity.validate()?;
        image.validate()?;
        if !self.removable {
            return Err("Wic writes require a removable whole device");
        }
        if !self.writable || self.read_only {
            return Err("the Wic write device is not writable");
        }
        if !self.descendant_mounts.is_empty() {
            return Err("the Wic write device has mounted descendants");
        }
        if self.identity.size_bytes < image.size_bytes {
            return Err("the Wic write device is smaller than the image");
        }
        if self.unavailable_reason.is_some() {
            return Err("the Wic write device is excluded by safety inspection");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDeviceInventoryRequest {
    pub generation: u64,
    pub image: WicOutputIdentity,
}

impl WicDeviceInventoryRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 {
            return Err("Wic device requests require a non-zero generation");
        }
        self.image.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WicDeviceInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: WicDeviceInventoryRequest,
    },
    Available {
        request: WicDeviceInventoryRequest,
        devices: Vec<WicDevice>,
    },
    Partial {
        request: WicDeviceInventoryRequest,
        devices: Vec<WicDevice>,
        limitations: Vec<String>,
    },
    Failed {
        request: WicDeviceInventoryRequest,
        message: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWriteRequest {
    pub executable: PathBuf,
    pub image: WicOutputIdentity,
    pub device: WicDeviceIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWritePreview {
    pub request: WicWriteRequest,
    pub argv: Vec<PathBuf>,
}

impl WicWritePreview {
    pub fn new(
        capability: &WicCapability,
        image: WicOutputIdentity,
        device: &WicDevice,
        phrase: &str,
    ) -> Result<Self, &'static str> {
        device.eligible_for(&image)?;
        let executable = match capability {
            WicCapability::Available { executable, .. } if absolute_normal_path(executable) => {
                executable.clone()
            }
            WicCapability::MissingKickstarts { executable } if absolute_normal_path(executable) => {
                executable.clone()
            }
            _ => return Err("Wic capability is unavailable for device writing"),
        };
        let expected = format!("WRITE {}", device.identity.path.display());
        if phrase != expected {
            return Err("Wic device confirmation phrase does not exactly match");
        }
        let request = WicWriteRequest {
            executable: executable.clone(),
            image,
            device: device.identity.clone(),
        };
        let argv = vec![
            executable,
            "write".into(),
            request.image.path.clone(),
            request.device.path.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDevicePickerDialog {
    pub request: WicDeviceInventoryRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWritePhraseDialog {
    pub request: WicDeviceInventoryRequest,
    pub device: WicDeviceIdentity,
    pub input: String,
    pub validation_error: Option<String>,
}

impl WicWritePhraseDialog {
    pub fn append(&mut self, character: char) {
        if !character.is_control()
            && self.input.len().saturating_add(character.len_utf8())
                <= MAX_WIC_WRITE_PHRASE_INPUT_BYTES
        {
            self.input.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.validation_error = None;
    }
}
