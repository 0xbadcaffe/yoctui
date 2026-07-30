use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_QA_CHECKS, MAX_QA_REPORT_PATHS, MAX_QA_SCOPES, QaCapabilitySnapshot, QaCheckAvailability,
    QaCheckCapability, QaCheckFamily, QaCheckId, QaScope, RecipeIdentity,
};

const MAX_QA_TASK_INPUTS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaFamilyTaskBinding {
    pub family: QaCheckFamily,
    pub task: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaReportRootInput {
    pub family: QaCheckFamily,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaTaskScopeInput {
    pub identity: RecipeIdentity,
    pub reported_tasks: Vec<String>,
    pub family_tasks: Vec<QaFamilyTaskBinding>,
    pub is_kernel: bool,
    pub report_roots: Vec<QaReportRootInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaTaskCapabilityInput {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected: RecipeIdentity,
    pub scopes: Vec<QaTaskScopeInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaTaskCapabilityResponse {
    Available(QaCapabilitySnapshot),
    Partial(QaCapabilitySnapshot),
}

impl QaTaskCapabilityResponse {
    pub fn snapshot(&self) -> &QaCapabilitySnapshot {
        match self {
            Self::Available(snapshot) | Self::Partial(snapshot) => snapshot,
        }
    }

    pub fn into_snapshot(self) -> QaCapabilitySnapshot {
        match self {
            Self::Available(snapshot) | Self::Partial(snapshot) => snapshot,
        }
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial(_))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QaTaskCapabilityError {
    #[error("QA build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("selected QA recipe/provider identity is unsafe")]
    UnsafeSelectedScope,
    #[error("selected QA recipe/provider is not in the eligible scope inventory")]
    MissingSelectedScope,
    #[error("too many QA capability inputs")]
    TooManyInputs,
    #[error("QA capability snapshot is invalid: {0}")]
    InvalidSnapshot(String),
}

#[derive(Debug, Clone)]
pub struct QaTaskCapabilityInspector {
    input: QaTaskCapabilityInput,
}

impl QaTaskCapabilityInspector {
    pub fn new(input: QaTaskCapabilityInput) -> Self {
        Self { input }
    }

    pub fn inspect(&self) -> Result<QaTaskCapabilityResponse, QaTaskCapabilityError> {
        validate_bounds(&self.input)?;
        let build_directory =
            canonical_directory(&self.input.build_directory).ok_or_else(|| {
                QaTaskCapabilityError::UnsafeBuildDirectory(self.input.build_directory.clone())
            })?;
        let selected = canonical_scope(&self.input.selected)
            .ok_or(QaTaskCapabilityError::UnsafeSelectedScope)?;
        let mut limitations = Vec::new();
        let mut scopes = Vec::new();
        let mut checks = Vec::new();
        let mut seen_scopes = BTreeSet::new();
        let mut selected_seen = false;

        for input in &self.input.scopes {
            let Some(scope) = canonical_scope(&input.identity) else {
                if input.identity == self.input.selected {
                    return Err(QaTaskCapabilityError::UnsafeSelectedScope);
                }
                limitations.push(format!(
                    "ignored unsafe QA provider scope: {} {}",
                    input.identity.name,
                    input.identity.file.display()
                ));
                continue;
            };
            if !seen_scopes.insert((scope.recipe.name.clone(), scope.recipe.file.clone())) {
                limitations.push(format!(
                    "ignored duplicate QA provider scope: {} {}",
                    scope.recipe.name,
                    scope.recipe.file.display()
                ));
                continue;
            }
            selected_seen |= scope == selected;
            let (scope_checks, mut scope_limitations) =
                inspect_scope(input, &scope, &build_directory);
            limitations.append(&mut scope_limitations);
            scopes.push(scope);
            checks.extend(scope_checks);
        }

        if !selected_seen {
            return Err(QaTaskCapabilityError::MissingSelectedScope);
        }
        limitations.sort();
        limitations.dedup();
        let snapshot = QaCapabilitySnapshot::new(
            self.input.release.clone(),
            build_directory,
            selected,
            scopes,
            checks,
            limitations.clone(),
        )
        .map_err(|message| QaTaskCapabilityError::InvalidSnapshot(message.into()))?;
        Ok(if limitations.is_empty() {
            QaTaskCapabilityResponse::Available(snapshot)
        } else {
            QaTaskCapabilityResponse::Partial(snapshot)
        })
    }
}

fn validate_bounds(input: &QaTaskCapabilityInput) -> Result<(), QaTaskCapabilityError> {
    if input.scopes.is_empty()
        || input.scopes.len() > MAX_QA_SCOPES
        || input.scopes.len().saturating_mul(required_families().len()) > MAX_QA_CHECKS
        || input.scopes.iter().any(|scope| {
            scope.reported_tasks.len() > MAX_QA_TASK_INPUTS
                || scope.family_tasks.len() > MAX_QA_CHECKS
                || scope.report_roots.len() > MAX_QA_REPORT_PATHS
        })
    {
        return Err(QaTaskCapabilityError::TooManyInputs);
    }
    Ok(())
}

fn inspect_scope(
    input: &QaTaskScopeInput,
    scope: &QaScope,
    build_directory: &Path,
) -> (Vec<QaCheckCapability>, Vec<String>) {
    let mut limitations = Vec::new();
    let mut reported = BTreeSet::new();
    for task in &input.reported_tasks {
        if !bounded_token(task) {
            limitations.push(format!(
                "ignored invalid reported QA task for {}",
                scope.recipe.name
            ));
        } else if !reported.insert(task.clone()) {
            limitations.push(format!(
                "ignored duplicate reported QA task {task} for {}",
                scope.recipe.name
            ));
        }
    }

    let mut bindings: BTreeMap<QaCheckFamily, Vec<String>> = BTreeMap::new();
    for binding in &input.family_tasks {
        if !bounded_token(&binding.task) {
            limitations.push(format!(
                "ignored invalid {:?} QA task binding for {}",
                binding.family, scope.recipe.name
            ));
            continue;
        }
        bindings
            .entry(binding.family)
            .or_default()
            .push(binding.task.clone());
    }
    for tasks in bindings.values_mut() {
        tasks.sort();
        tasks.dedup();
    }

    let mut roots: BTreeMap<QaCheckFamily, Vec<PathBuf>> = BTreeMap::new();
    for root in &input.report_roots {
        match canonical_directory(&root.path) {
            Some(path) if path.starts_with(build_directory) => {
                let values = roots.entry(root.family).or_default();
                if !values.contains(&path) {
                    values.push(path);
                }
            }
            Some(_) | None => limitations.push(format!(
                "ignored unsafe {:?} QA report root for {}: {}",
                root.family,
                scope.recipe.name,
                root.path.display()
            )),
        }
    }
    for values in roots.values_mut() {
        values.sort();
    }

    let checks = required_families()
        .into_iter()
        .map(|(family, id, label)| {
            let family_bindings = bindings.get(&family).map(Vec::as_slice).unwrap_or_default();
            let (task, availability) =
                resolve_task(family, input.is_kernel, family_bindings, &reported);
            QaCheckCapability::new(
                QaCheckId::new(id.into()).expect("static QA check IDs are valid"),
                family,
                label.into(),
                scope.clone(),
                task,
                roots.remove(&family).unwrap_or_default(),
                availability,
                Vec::new(),
            )
            .expect("validated adapter inputs produce a valid QA check")
        })
        .collect();
    (checks, limitations)
}

fn resolve_task(
    family: QaCheckFamily,
    is_kernel: bool,
    bindings: &[String],
    reported: &BTreeSet<String>,
) -> (Option<String>, QaCheckAvailability) {
    if family == QaCheckFamily::KernelConfiguration && !is_kernel {
        return (
            None,
            QaCheckAvailability::Disabled(
                "kernel configuration checks require authoritative kernel classification".into(),
            ),
        );
    }
    match bindings {
        [] => (
            None,
            QaCheckAvailability::Disabled(
                "no authoritative task is bound to this QA family".into(),
            ),
        ),
        [task] if reported.contains(task) => (Some(task.clone()), QaCheckAvailability::Available),
        [..] if bindings.len() > 1 => (
            None,
            QaCheckAvailability::Disabled(
                "multiple authoritative tasks are bound to this QA family".into(),
            ),
        ),
        [..] => (
            None,
            QaCheckAvailability::Disabled(
                "the bound task is not reported for the exact recipe scope".into(),
            ),
        ),
    }
}

fn required_families() -> [(QaCheckFamily, &'static str, &'static str); 5] {
    [
        (
            QaCheckFamily::KernelConfiguration,
            "kernel-configuration",
            "Kernel configuration",
        ),
        (
            QaCheckFamily::UriFetch,
            "uri-fetch",
            "URI and fetch metadata",
        ),
        (
            QaCheckFamily::Patch,
            "patch",
            "Patch metadata and application",
        ),
        (
            QaCheckFamily::License,
            "license",
            "License metadata and checksums",
        ),
        (
            QaCheckFamily::RecipePackage,
            "recipe-package",
            "Recipe and package QA",
        ),
    ]
}

fn canonical_scope(identity: &RecipeIdentity) -> Option<QaScope> {
    if !bounded_token(&identity.name) || !canonical_regular_file(&identity.file) {
        return None;
    }
    QaScope::new(identity.clone()).ok()
}

fn canonical_regular_file(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    !metadata.file_type().is_symlink()
        && metadata.is_file()
        && fs::canonicalize(path).ok().as_ref() == Some(&path.to_path_buf())
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    (canonical == path).then_some(canonical)
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{BuildRequest, QaCheckAvailability};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "yoctui-qa-task-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn directory(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir_all(&path).unwrap();
            fs::canonicalize(path).unwrap()
        }

        fn provider(&self, name: &str) -> PathBuf {
            let directory = self.directory("providers");
            let path = directory.join(format!("{name}.bb"));
            fs::write(&path, b"SUMMARY = \"fixture\"\n").unwrap();
            fs::canonicalize(path).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn identity(name: &str, provider: PathBuf) -> RecipeIdentity {
        RecipeIdentity {
            name: name.into(),
            file: provider,
        }
    }

    fn binding(family: QaCheckFamily, task: &str) -> QaFamilyTaskBinding {
        QaFamilyTaskBinding {
            family,
            task: task.into(),
        }
    }

    fn scope_input(
        identity: RecipeIdentity,
        is_kernel: bool,
        report_root: PathBuf,
    ) -> QaTaskScopeInput {
        let tasks = vec![
            "do_kernel_configcheck",
            "do_checkuri",
            "do_patch_qa",
            "do_populate_lic",
            "do_package_qa",
        ];
        QaTaskScopeInput {
            identity,
            reported_tasks: tasks.into_iter().map(str::to_owned).collect(),
            family_tasks: vec![
                binding(QaCheckFamily::KernelConfiguration, "do_kernel_configcheck"),
                binding(QaCheckFamily::UriFetch, "do_checkuri"),
                binding(QaCheckFamily::Patch, "do_patch_qa"),
                binding(QaCheckFamily::License, "do_populate_lic"),
                binding(QaCheckFamily::RecipePackage, "do_package_qa"),
            ],
            is_kernel,
            report_roots: vec![QaReportRootInput {
                family: QaCheckFamily::KernelConfiguration,
                path: report_root,
            }],
        }
    }

    fn input(fixture: &Fixture) -> QaTaskCapabilityInput {
        let build = fixture.directory("build");
        let reports = fixture.directory("build/reports");
        let kernel = identity("linux-yocto", fixture.provider("linux-yocto"));
        let busybox = identity("busybox", fixture.provider("busybox"));
        QaTaskCapabilityInput {
            release: Some("6.0".into()),
            build_directory: build,
            selected: kernel.clone(),
            scopes: vec![
                scope_input(kernel, true, reports.clone()),
                scope_input(busybox, false, reports),
            ],
        }
    }

    #[test]
    fn qa_task_capability_uses_only_exact_family_bindings_and_reported_tasks() {
        let fixture = Fixture::new();
        let response = QaTaskCapabilityInspector::new(input(&fixture))
            .inspect()
            .unwrap();
        assert!(!response.is_partial());
        let snapshot = response.snapshot();
        assert_eq!(snapshot.scopes.len(), 2);
        assert_eq!(snapshot.checks.len(), 10);
        let kernel = snapshot
            .checks
            .iter()
            .find(|check| {
                check.scope.recipe.name == "linux-yocto"
                    && check.family == QaCheckFamily::KernelConfiguration
            })
            .unwrap();
        assert_eq!(kernel.task.as_deref(), Some("do_kernel_configcheck"));
        assert!(matches!(
            kernel.availability,
            QaCheckAvailability::Available
        ));
        assert_eq!(kernel.report_roots.len(), 1);
        let request = BuildRequest {
            targets: vec![kernel.scope.recipe.name.clone()],
            task: kernel.task.clone(),
            force: false,
        };
        request.validate().unwrap();
        assert_eq!(request.task.as_deref(), Some("do_kernel_configcheck"));

        let non_kernel = snapshot
            .checks
            .iter()
            .find(|check| {
                check.scope.recipe.name == "busybox"
                    && check.family == QaCheckFamily::KernelConfiguration
            })
            .unwrap();
        assert!(non_kernel.task.is_none());
        assert_eq!(
            non_kernel.availability.disabled_reason(),
            Some("kernel configuration checks require authoritative kernel classification")
        );
    }

    #[test]
    fn qa_task_capability_accepts_alternate_tasks_only_as_explicit_data() {
        let fixture = Fixture::new();
        let mut input = input(&fixture);
        let kernel = &mut input.scopes[0];
        kernel.reported_tasks.push("do_vendor_uri_audit".into());
        kernel
            .family_tasks
            .retain(|binding| binding.family != QaCheckFamily::UriFetch);
        kernel
            .family_tasks
            .push(binding(QaCheckFamily::UriFetch, "do_vendor_uri_audit"));
        let snapshot = QaTaskCapabilityInspector::new(input)
            .inspect()
            .unwrap()
            .into_snapshot();
        let uri = snapshot
            .checks
            .iter()
            .find(|check| {
                check.scope.recipe.name == "linux-yocto" && check.family == QaCheckFamily::UriFetch
            })
            .unwrap();
        assert_eq!(uri.task.as_deref(), Some("do_vendor_uri_audit"));
    }

    #[test]
    fn qa_task_capability_never_guesses_missing_or_similar_tasks() {
        let fixture = Fixture::new();
        let mut input = input(&fixture);
        let kernel = &mut input.scopes[0];
        kernel.reported_tasks.retain(|task| task != "do_checkuri");
        kernel.reported_tasks.push("do_checkuris".into());
        let snapshot = QaTaskCapabilityInspector::new(input)
            .inspect()
            .unwrap()
            .into_snapshot();
        let uri = snapshot
            .checks
            .iter()
            .find(|check| {
                check.scope.recipe.name == "linux-yocto" && check.family == QaCheckFamily::UriFetch
            })
            .unwrap();
        assert!(uri.task.is_none());
        assert_eq!(
            uri.availability.disabled_reason(),
            Some("the bound task is not reported for the exact recipe scope")
        );
    }

    #[test]
    fn qa_task_capability_preserves_usable_inputs_as_partial() {
        let fixture = Fixture::new();
        let mut input = input(&fixture);
        let unsafe_provider = fixture.root.join("missing.bb");
        input.scopes.push(scope_input(
            identity("optional", unsafe_provider),
            false,
            fixture.root.join("missing-reports"),
        ));
        input.scopes[0].report_roots.push(QaReportRootInput {
            family: QaCheckFamily::License,
            path: fixture.directory("outside-license-reports"),
        });
        input.scopes[0].reported_tasks.push("bad/task".into());
        let response = QaTaskCapabilityInspector::new(input).inspect().unwrap();
        assert!(response.is_partial());
        assert_eq!(response.snapshot().scopes.len(), 2);
        assert!(
            response
                .snapshot()
                .limitations
                .iter()
                .any(|value| { value.contains("unsafe QA provider scope") })
        );
        assert!(
            response
                .snapshot()
                .limitations
                .iter()
                .any(|value| { value.contains("unsafe License QA report root") })
        );
    }

    #[test]
    fn qa_task_capability_rejects_stale_selected_provider_and_symlinks() {
        let fixture = Fixture::new();
        let stale = input(&fixture);
        fs::remove_file(&stale.selected.file).unwrap();
        assert_eq!(
            QaTaskCapabilityInspector::new(stale).inspect(),
            Err(QaTaskCapabilityError::UnsafeSelectedScope)
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let mut linked = input(&fixture);
            let real = linked.selected.file.clone();
            let link = fixture.root.join("linked-provider.bb");
            symlink(real, &link).unwrap();
            linked.selected.file = link.clone();
            linked.scopes[0].identity.file = link;
            assert_eq!(
                QaTaskCapabilityInspector::new(linked).inspect(),
                Err(QaTaskCapabilityError::UnsafeSelectedScope)
            );
        }
    }

    #[test]
    fn qa_task_capability_handles_duplicates_ambiguity_and_bounds() {
        let fixture = Fixture::new();
        let mut capability_input = input(&fixture);
        capability_input
            .scopes
            .push(capability_input.scopes[1].clone());
        capability_input.scopes[0]
            .family_tasks
            .push(binding(QaCheckFamily::License, "do_license_alt"));
        capability_input.scopes[0]
            .reported_tasks
            .push("do_license_alt".into());
        let response = QaTaskCapabilityInspector::new(capability_input)
            .inspect()
            .unwrap();
        assert!(response.is_partial());
        let license = response
            .snapshot()
            .checks
            .iter()
            .find(|check| {
                check.scope.recipe.name == "linux-yocto" && check.family == QaCheckFamily::License
            })
            .unwrap();
        assert!(license.task.is_none());
        assert_eq!(
            license.availability.disabled_reason(),
            Some("multiple authoritative tasks are bound to this QA family")
        );
        assert!(
            response
                .snapshot()
                .limitations
                .iter()
                .any(|value| { value.contains("duplicate QA provider scope") })
        );

        let mut oversized = input(&fixture);
        oversized.scopes[0].reported_tasks = (0..=MAX_QA_TASK_INPUTS)
            .map(|index| format!("task{index}"))
            .collect();
        assert_eq!(
            QaTaskCapabilityInspector::new(oversized).inspect(),
            Err(QaTaskCapabilityError::TooManyInputs)
        );
    }
}
