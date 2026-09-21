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
#[path = "tests/qa_task/mod.rs"]
mod tests;
