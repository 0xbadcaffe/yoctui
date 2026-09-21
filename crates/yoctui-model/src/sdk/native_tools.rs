#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkNativeMode {
    FindSysroot,
    RunNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SdkToolCapability {
    #[default]
    NotInspected,
    Available {
        publish: Option<PathBuf>,
        find_sysroot: Option<PathBuf>,
        run_native: Option<PathBuf>,
    },
    Failed {
        message: String,
    },
}

impl SdkToolCapability {
    pub fn executable_for(&self, mode: SdkNativeMode) -> Result<PathBuf, &'static str> {
        let Self::Available {
            find_sysroot,
            run_native,
            ..
        } = self
        else {
            return Err("SDK native-tool capability is unavailable");
        };
        match mode {
            SdkNativeMode::FindSysroot => find_sysroot.clone(),
            SdkNativeMode::RunNative => run_native.clone(),
        }
        .filter(|path| absolute_normal_path(path))
        .ok_or("the requested SDK native tool is unavailable")
    }

    pub fn publish_executable(&self) -> Result<PathBuf, &'static str> {
        let Self::Available { publish, .. } = self else {
            return Err("SDK publication capability is unavailable");
        };
        publish
            .clone()
            .filter(|path| absolute_normal_path(path))
            .ok_or("oe-publish-sdk is unavailable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativeDraft {
    pub mode: SdkNativeMode,
    pub extracted_root: String,
    pub recipe: String,
    pub tool: String,
    pub arguments: Vec<String>,
}

impl Default for SdkNativeDraft {
    fn default() -> Self {
        Self {
            mode: SdkNativeMode::FindSysroot,
            extracted_root: String::new(),
            recipe: String::new(),
            tool: String::new(),
            arguments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkNativeField {
    Mode,
    Workspace,
    Recipe,
    Tool,
    Arguments,
}

impl SdkNativeField {
    const ALL: [Self; 5] = [
        Self::Mode,
        Self::Workspace,
        Self::Recipe,
        Self::Tool,
        Self::Arguments,
    ];

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(Self::ALL.len() - 1)
        };
        Self::ALL[next]
    }

    pub fn is_text(self) -> bool {
        !matches!(self, Self::Mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativeDialog {
    pub draft: SdkNativeDraft,
    pub selected_field: SdkNativeField,
    pub editing: bool,
    pub arguments_input: String,
    pub validation_error: Option<String>,
}

impl SdkNativeDialog {
    pub fn new(draft: SdkNativeDraft) -> Self {
        let arguments_input = draft.arguments.join(" ");
        Self {
            draft,
            selected_field: SdkNativeField::Mode,
            editing: false,
            arguments_input,
            validation_error: None,
        }
    }

    pub fn selected_text_mut(&mut self) -> Option<(&mut String, usize)> {
        match self.selected_field {
            SdkNativeField::Workspace => Some((&mut self.draft.extracted_root, 4_096)),
            SdkNativeField::Recipe => Some((&mut self.draft.recipe, 256)),
            SdkNativeField::Tool => Some((&mut self.draft.tool, 256)),
            SdkNativeField::Arguments => Some((
                &mut self.arguments_input,
                MAX_SDK_NATIVE_ARGUMENT_INPUT_BYTES,
            )),
            SdkNativeField::Mode => None,
        }
    }

    pub fn cycle_mode(&mut self) {
        self.draft.mode = match self.draft.mode {
            SdkNativeMode::FindSysroot => SdkNativeMode::RunNative,
            SdkNativeMode::RunNative => SdkNativeMode::FindSysroot,
        };
        self.validation_error = None;
    }

    pub fn synchronize_arguments(&mut self) {
        self.draft.arguments = self
            .arguments_input
            .split_ascii_whitespace()
            .take(MAX_SDK_NATIVE_ARGUMENTS + 1)
            .map(str::to_owned)
            .collect();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativeRequest {
    pub executable: PathBuf,
    pub mode: SdkNativeMode,
    pub extracted_root: Option<PathBuf>,
    pub recipe: String,
    pub tool: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkNativePreview {
    pub request: SdkNativeRequest,
    pub argv: Vec<PathBuf>,
}

impl SdkNativePreview {
    pub fn new(request: SdkNativeRequest) -> Result<Self, &'static str> {
        if !absolute_normal_path(&request.executable)
            || !token_is_valid(&request.recipe)
            || request
                .extracted_root
                .as_ref()
                .is_some_and(|path| !absolute_normal_path(path))
            || request.arguments.len() > MAX_SDK_NATIVE_ARGUMENTS
            || request.arguments.iter().any(|argument| {
                argument.is_empty()
                    || argument.len() > MAX_SDK_NATIVE_ARGUMENT_BYTES
                    || argument.chars().any(char::is_control)
            })
        {
            return Err("SDK native-tool request is invalid or exceeds its bound");
        }
        match request.mode {
            SdkNativeMode::FindSysroot
                if request.tool.is_some() || !request.arguments.is_empty() =>
            {
                return Err("find-native-sysroot does not accept a tool or tool arguments");
            }
            SdkNativeMode::RunNative
                if request
                    .tool
                    .as_deref()
                    .is_none_or(|tool| !token_is_valid(tool)) =>
            {
                return Err("run-native requires a bounded tool token");
            }
            _ => {}
        }
        let mut argv = vec![request.executable.clone(), request.recipe.clone().into()];
        if let Some(tool) = &request.tool {
            argv.push(tool.into());
        }
        argv.extend(request.arguments.iter().map(PathBuf::from));
        Ok(Self { request, argv })
    }
}
