#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSelftestRequest {
    pub executable: PathBuf,
    pub family: TestFamily,
    pub selector: Option<String>,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

impl TestSelftestRequest {
    pub fn new(
        executable: PathBuf,
        family: TestFamily,
        selector: Option<String>,
        parallelism: u16,
        verbose: bool,
        skip_network: bool,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable)
            || !matches!(family, TestFamily::OeSelftest | TestFamily::BitbakeSelftest)
            || selector
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || parallelism == 0
            || parallelism > 256
        {
            return Err("selftest request is invalid or exceeds its bound");
        }
        Ok(Self {
            executable,
            family,
            selector,
            parallelism,
            verbose,
            skip_network,
        })
    }

    pub fn argv(&self) -> Vec<PathBuf> {
        let mut argv = vec![self.executable.clone()];
        match self.family {
            TestFamily::OeSelftest => {
                if let Some(selector) = &self.selector {
                    argv.push("-r".into());
                    argv.push(selector.into());
                } else {
                    argv.push("-a".into());
                }
                argv.push("-j".into());
                argv.push(self.parallelism.to_string().into());
            }
            TestFamily::BitbakeSelftest => {
                if self.verbose {
                    argv.push("-v".into());
                }
                if let Some(selector) = &self.selector {
                    argv.push(selector.into());
                }
            }
            _ => {}
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestLaunchPreview {
    Selftest(TestSelftestRequest),
    Build {
        family: TestFamily,
        machine: String,
        distro: String,
        image: String,
        request: BuildRequest,
    },
}

impl TestLaunchPreview {
    pub fn family(&self) -> TestFamily {
        match self {
            Self::Selftest(request) => request.family,
            Self::Build { family, .. } => *family,
        }
    }

    pub fn operation(&self) -> TestOperation {
        match self {
            Self::Selftest(request) => TestOperation::Selftest(request.clone()),
            Self::Build {
                family, request, ..
            } => TestOperation::Build {
                family: *family,
                request: request.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestLaunchField {
    Scope,
    Selector,
    Parallelism,
    Verbose,
    SkipNetwork,
}

impl TestLaunchField {
    fn fields(family: TestFamily) -> &'static [Self] {
        match family {
            TestFamily::OeSelftest => &[Self::Scope, Self::Selector, Self::Parallelism],
            TestFamily::BitbakeSelftest => &[
                Self::Scope,
                Self::Selector,
                Self::Verbose,
                Self::SkipNetwork,
            ],
            _ => &[],
        }
    }

    pub fn shifted(self, family: TestFamily, delta: isize) -> Self {
        let fields = Self::fields(family);
        let current = fields
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(fields.len().saturating_sub(1))
        };
        fields.get(next).copied().unwrap_or(self)
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Selector | Self::Parallelism)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLaunchDialog {
    pub draft: TestLaunchDraft,
    pub selected_field: Option<TestLaunchField>,
    pub editing: bool,
    pub parallelism_input: String,
    pub validation_error: Option<String>,
}

impl TestLaunchDialog {
    pub fn new(draft: TestLaunchDraft) -> Self {
        let selected_field = TestLaunchField::fields(draft.family).first().copied();
        Self {
            parallelism_input: draft.parallelism.to_string(),
            draft,
            selected_field,
            editing: false,
            validation_error: None,
        }
    }

    pub fn select(&mut self, delta: isize) {
        if let Some(field) = self.selected_field {
            self.selected_field = Some(field.shifted(self.draft.family, delta));
        }
        self.validation_error = None;
    }

    pub fn activate(&mut self) {
        match self.selected_field {
            Some(TestLaunchField::Scope) => {
                self.draft.scope = match self.draft.scope {
                    TestSelectorScope::All => TestSelectorScope::Selected,
                    TestSelectorScope::Selected => TestSelectorScope::All,
                };
                if self.draft.scope == TestSelectorScope::All {
                    self.draft.selector.clear();
                }
            }
            Some(TestLaunchField::Selector | TestLaunchField::Parallelism) => {
                self.editing = true;
            }
            Some(TestLaunchField::Verbose) => self.draft.verbose = !self.draft.verbose,
            Some(TestLaunchField::SkipNetwork) => {
                self.draft.skip_network = !self.draft.skip_network;
            }
            None => {}
        }
        self.validation_error = None;
    }

    pub fn append(&mut self, character: char) {
        if !self.editing || character.is_control() {
            return;
        }
        match self.selected_field {
            Some(TestLaunchField::Selector)
                if self.draft.selector.len() + character.len_utf8() <= MAX_TEST_SELECTOR_BYTES =>
            {
                self.draft.selector.push(character);
            }
            Some(TestLaunchField::Parallelism)
                if character.is_ascii_digit()
                    && self.parallelism_input.len() < MAX_TEST_PARALLELISM_INPUT_BYTES =>
            {
                self.parallelism_input.push(character);
            }
            _ => {}
        }
        self.validation_error = None;
    }

    pub fn backspace(&mut self) {
        if !self.editing {
            return;
        }
        match self.selected_field {
            Some(TestLaunchField::Selector) => {
                self.draft.selector.pop();
            }
            Some(TestLaunchField::Parallelism) => {
                self.parallelism_input.pop();
            }
            _ => {}
        }
        self.validation_error = None;
    }

    pub fn finish_edit(&mut self) {
        if self.selected_field == Some(TestLaunchField::Parallelism) {
            match self.parallelism_input.parse::<u16>() {
                Ok(value @ 1..=256) => self.draft.parallelism = value,
                _ => {
                    self.validation_error = Some("Parallelism must be between 1 and 256.".into());
                    return;
                }
            }
        }
        self.editing = false;
    }
}
