#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitDestinationInspection {
    pub requested: PathBuf,
    pub canonical_parent: Option<PathBuf>,
    pub parent_exists: bool,
    pub parent_is_directory: bool,
    pub destination_exists: bool,
    pub destination_is_symlink: bool,
}

impl TestJunitDestinationInspection {
    pub fn validated_destination(&self) -> Result<PathBuf, &'static str> {
        let parent = self
            .canonical_parent
            .as_deref()
            .ok_or("JUnit destination parent is not canonical")?;
        let requested_parent = self
            .requested
            .parent()
            .ok_or("JUnit destination has no parent")?;
        if !absolute_normal_path(&self.requested)
            || self.requested.extension().and_then(|value| value.to_str()) != Some("xml")
            || !self.parent_exists
            || !self.parent_is_directory
            || self.destination_exists
            || self.destination_is_symlink
            || requested_parent != parent
        {
            return Err("JUnit destination is unsafe or would overwrite an existing path");
        }
        Ok(self.requested.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportRequest {
    pub generation: u64,
    pub result: TestResultIdentity,
    pub destination: PathBuf,
}

impl TestJunitExportRequest {
    pub fn new(
        generation: u64,
        result: TestResultIdentity,
        inspection: &TestJunitDestinationInspection,
    ) -> Result<Self, &'static str> {
        if generation == 0 {
            return Err("JUnit export generation is invalid");
        }
        Ok(Self {
            generation,
            result,
            destination: inspection.validated_destination()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportPreview {
    pub request: TestJunitExportRequest,
    pub argv: Vec<PathBuf>,
}

impl TestJunitExportPreview {
    pub fn new(executable: PathBuf, request: TestJunitExportRequest) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable) {
            return Err("resulttool executable identity is invalid");
        }
        let argv = vec![
            executable,
            "junit".into(),
            request.result.path.clone(),
            "-j".into(),
            request.destination.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestJunitExportState {
    #[default]
    NotStarted,
    Inspecting {
        result: TestResultIdentity,
        destination: PathBuf,
    },
    Ready(TestJunitExportPreview),
    Running(TestJunitExportRequest),
    Succeeded(TestJunitExportRequest),
    Failed {
        request: TestJunitExportRequest,
        message: String,
    },
    Cancelled(TestJunitExportRequest),
    TimedOut(TestJunitExportRequest),
    Lost {
        request: TestJunitExportRequest,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportDialog {
    pub result: TestResultIdentity,
    pub destination_input: String,
    pub validation_error: Option<String>,
}

impl TestJunitExportDialog {
    pub fn new(result: TestResultIdentity) -> Self {
        Self {
            result,
            destination_input: String::new(),
            validation_error: None,
        }
    }

    pub fn append(&mut self, character: char) {
        if !character.is_control()
            && self.destination_input.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
        {
            self.destination_input.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.destination_input.pop();
        self.validation_error = None;
    }

    pub fn lexical_destination(&self) -> Result<PathBuf, &'static str> {
        let path = PathBuf::from(&self.destination_input);
        if !absolute_normal_path(&path)
            || path.extension().and_then(|value| value.to_str()) != Some("xml")
        {
            return Err("JUnit destination must be a normalized absolute .xml path");
        }
        Ok(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestFamily {
    OeSelftest,
    BitbakeSelftest,
    TestImage,
    TestSdk,
    TestSdkExt,
    Ptest,
}

impl TestFamily {
    pub const ALL: [Self; 6] = [
        Self::OeSelftest,
        Self::BitbakeSelftest,
        Self::TestImage,
        Self::TestSdk,
        Self::TestSdkExt,
        Self::Ptest,
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

    pub fn task(self) -> Option<&'static str> {
        match self {
            Self::TestImage | Self::Ptest => Some("testimage"),
            Self::TestSdk => Some("testsdk"),
            Self::TestSdkExt => Some("testsdkext"),
            Self::OeSelftest | Self::BitbakeSelftest => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OeSelftest => "OE selftest",
            Self::BitbakeSelftest => "BitBake selftest",
            Self::TestImage => "Image runtime",
            Self::TestSdk => "Standard SDK",
            Self::TestSdkExt => "Extensible SDK",
            Self::Ptest => "Package tests",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestExecutableCapability {
    #[default]
    NotInspected,
    Missing,
    Available(PathBuf),
    Failed(String),
}

impl TestExecutableCapability {
    pub fn executable(&self) -> Result<PathBuf, &'static str> {
        match self {
            Self::Available(path) if absolute_normal_path(path) => Ok(path.clone()),
            Self::Available(_) => Err("test executable identity is invalid"),
            Self::NotInspected => Err("test executable has not been inspected"),
            Self::Missing => Err("test executable is missing"),
            Self::Failed(_) => Err("test executable inspection failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PtestCapability {
    #[default]
    NotInspected,
    Configured,
    Unavailable(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestCapability {
    pub oe_selftest: TestExecutableCapability,
    pub bitbake_selftest: TestExecutableCapability,
    pub ptest: PtestCapability,
}

impl TestCapability {
    pub fn executable_for(&self, family: TestFamily) -> Result<PathBuf, &'static str> {
        match family {
            TestFamily::OeSelftest => self.oe_selftest.executable(),
            TestFamily::BitbakeSelftest => self.bitbake_selftest.executable(),
            _ => Err("the selected test family is a managed BitBake task"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSelectorScope {
    All,
    Selected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLaunchDraft {
    pub family: TestFamily,
    pub machine: String,
    pub distro: String,
    pub image: String,
    pub scope: TestSelectorScope,
    pub selector: String,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

impl TestLaunchDraft {
    pub fn new(family: TestFamily, machine: String, distro: String, image: String) -> Self {
        Self {
            family,
            machine,
            distro,
            image,
            scope: TestSelectorScope::All,
            selector: String::new(),
            parallelism: 1,
            verbose: false,
            skip_network: false,
        }
    }

    pub fn preview(&self, capability: &TestCapability) -> Result<TestLaunchPreview, &'static str> {
        match self.family {
            TestFamily::OeSelftest | TestFamily::BitbakeSelftest => {
                let executable = capability.executable_for(self.family)?;
                if self.parallelism == 0 || self.parallelism > 256 {
                    return Err("test parallelism must be between 1 and 256");
                }
                let selector = match self.scope {
                    TestSelectorScope::All => None,
                    TestSelectorScope::Selected if bounded_token(&self.selector) => {
                        Some(self.selector.clone())
                    }
                    TestSelectorScope::Selected => {
                        return Err("selected test identity is invalid or unavailable");
                    }
                };
                if self.family == TestFamily::BitbakeSelftest
                    && self.scope == TestSelectorScope::All
                    && !self.selector.is_empty()
                {
                    return Err("BitBake selftest selector state is inconsistent");
                }
                TestSelftestRequest::new(
                    executable,
                    self.family,
                    selector,
                    self.parallelism,
                    self.verbose,
                    self.skip_network,
                )
                .map(TestLaunchPreview::Selftest)
            }
            family => {
                if !bounded_token(&self.machine)
                    || !bounded_token(&self.distro)
                    || !bounded_token(&self.image)
                {
                    return Err("test build identity must use bounded BitBake tokens");
                }
                if family == TestFamily::Ptest
                    && !matches!(capability.ptest, PtestCapability::Configured)
                {
                    return Err("ptest is not confirmed in the active image configuration");
                }
                let request = BuildRequest {
                    targets: vec![self.image.clone()],
                    task: family.task().map(str::to_owned),
                    force: false,
                };
                request
                    .validate()
                    .map_err(|_| "test BuildRequest is invalid")?;
                Ok(TestLaunchPreview::Build {
                    family,
                    machine: self.machine.clone(),
                    distro: self.distro.clone(),
                    image: self.image.clone(),
                    request,
                })
            }
        }
    }
}
