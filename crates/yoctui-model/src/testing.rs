use crate::{BackgroundJobId, BuildRequest};
use std::{
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

pub const MAX_TEST_SELECTOR_BYTES: usize = 256;
pub const MAX_TEST_PARALLELISM_INPUT_BYTES: usize = 3;
pub const MAX_TEST_RESULT_PATHS: usize = 256;

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEST_SELECTOR_BYTES
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= 4_096
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

pub fn test_result_paths_are_valid(paths: &[PathBuf]) -> bool {
    paths.len() <= MAX_TEST_RESULT_PATHS && paths.iter().all(|path| absolute_normal_path(path))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestOperation {
    Selftest(TestSelftestRequest),
    Build {
        family: TestFamily,
        request: BuildRequest,
    },
}

impl TestOperation {
    pub fn family(&self) -> TestFamily {
        match self {
            Self::Selftest(request) => request.family,
            Self::Build { family, .. } => *family,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSession {
    pub id: TestSessionId,
    pub background_job_id: Option<BackgroundJobId>,
    pub operation: TestOperation,
    pub exit_code: Option<i32>,
    pub result_paths: Vec<PathBuf>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSessionTerminal {
    pub id: TestSessionId,
    pub exit_code: Option<i32>,
    pub result_paths: Vec<PathBuf>,
    pub finished_at: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability() -> TestCapability {
        TestCapability {
            oe_selftest: TestExecutableCapability::Available("/workspace/oe-selftest".into()),
            bitbake_selftest: TestExecutableCapability::Available(
                "/workspace/bitbake-selftest".into(),
            ),
            ptest: PtestCapability::Configured,
        }
    }

    #[test]
    fn test_workflow_previews_exact_selftest_and_build_operations() {
        let mut oe = TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
        );
        oe.scope = TestSelectorScope::Selected;
        oe.selector = "tinfoil.TinfoilTests.test_getvar".into();
        oe.parallelism = 8;
        let TestLaunchPreview::Selftest(request) = oe.preview(&capability()).unwrap() else {
            panic!("selftest preview");
        };
        assert_eq!(
            request.argv(),
            [
                PathBuf::from("/workspace/oe-selftest"),
                "-r".into(),
                "tinfoil.TinfoilTests.test_getvar".into(),
                "-j".into(),
                "8".into(),
            ]
        );

        for (family, task) in [
            (TestFamily::TestImage, "testimage"),
            (TestFamily::TestSdk, "testsdk"),
            (TestFamily::TestSdkExt, "testsdkext"),
            (TestFamily::Ptest, "testimage"),
        ] {
            let TestLaunchPreview::Build { request, .. } = TestLaunchDraft::new(
                family,
                "qemux86-64".into(),
                "poky".into(),
                "core-image-minimal".into(),
            )
            .preview(&capability())
            .unwrap() else {
                panic!("build preview");
            };
            assert_eq!(request.task.as_deref(), Some(task));
            assert_eq!(request.targets, ["core-image-minimal"]);
        }
    }

    #[test]
    fn test_workflow_dialog_bounds_editing_and_reports_invalid_state() {
        let mut dialog = TestLaunchDialog::new(TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
        ));
        dialog.activate();
        dialog.select(1);
        dialog.activate();
        for _ in 0..(MAX_TEST_SELECTOR_BYTES + 10) {
            dialog.append('x');
        }
        assert_eq!(dialog.draft.selector.len(), MAX_TEST_SELECTOR_BYTES);
        dialog.finish_edit();
        dialog.select(1);
        dialog.activate();
        dialog.parallelism_input.clear();
        for character in "999".chars() {
            dialog.append(character);
        }
        dialog.finish_edit();
        assert!(dialog.editing);
        assert!(dialog.validation_error.is_some());
    }

    #[test]
    fn test_workflow_rejects_missing_tools_bad_tokens_and_unconfigured_ptest() {
        let missing = TestCapability::default();
        let oe = TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "image".into(),
        );
        assert_eq!(
            oe.preview(&missing),
            Err("test executable has not been inspected")
        );
        let bad = TestLaunchDraft::new(
            TestFamily::TestImage,
            "../machine".into(),
            "poky".into(),
            "image".into(),
        );
        assert!(bad.preview(&capability()).is_err());
        let ptest = TestLaunchDraft::new(
            TestFamily::Ptest,
            "qemux86-64".into(),
            "poky".into(),
            "image".into(),
        );
        assert!(ptest.preview(&missing).is_err());
    }
}
