//! Recipe render.
use super::*;

pub(crate) fn recipe_build_state(app: &App, recipe: &str) -> String {
    if let Some(task) = app.tasks.values().find(|task| task.recipe == recipe) {
        return format!("{:?}", task.state).to_ascii_lowercase();
    }
    if let Some(task) = app
        .completed_tasks
        .iter()
        .find(|task| task.task.recipe == recipe)
    {
        return format!("{:?}", task.task.state).to_ascii_lowercase();
    }
    if app.build.target.as_deref() == Some(recipe) {
        return match app.build.status {
            BuildStatus::Idle | BuildStatus::LoadingWorkspace => "idle",
            BuildStatus::Parsing => "parsing",
            BuildStatus::Running => "running",
            BuildStatus::Cancelling => "cancelling",
            BuildStatus::Completed => "succeeded",
            BuildStatus::Cancelled => "cancelled",
            BuildStatus::Failed => "failed",
            BuildStatus::Lost => "lost",
        }
        .into();
    }
    app.recipe_metadata
        .get(recipe)
        .and_then(|metadata| metadata.build_status)
        .map_or_else(
            || "unavailable".into(),
            |status| {
                match status {
                    RecipeBuildStatus::Idle => "idle",
                    RecipeBuildStatus::Queued => "queued",
                    RecipeBuildStatus::Running => "running",
                    RecipeBuildStatus::Succeeded => "succeeded",
                    RecipeBuildStatus::Failed => "failed",
                    RecipeBuildStatus::Cancelled => "cancelled",
                }
                .into()
            },
        )
}

pub(crate) fn recipe_workspace_state(app: &App, recipe: &Recipe) -> String {
    let identity = recipe.file.as_ref().and_then(|file| {
        file.is_absolute().then(|| RecipeIdentity {
            name: recipe.name.clone(),
            file: file.clone(),
        })
    });
    let Some(identity) = identity else {
        return "unavailable (absolute provider path not reported)".into();
    };
    if app.devtool_status_loading.contains(&identity) {
        return "loading authoritative status…".into();
    }
    let Some(status) = app.devtool_statuses.get(&identity) else {
        return "not inspected; press t (or Enter)".into();
    };
    match &status.capability {
        DevtoolCapability::Available => {}
        DevtoolCapability::MissingExecutable => {
            return "Devtool executable missing; all actions disabled".into();
        }
        DevtoolCapability::Unavailable { reason } => {
            return format!("{reason}; all actions disabled");
        }
    }
    if let Some(error) = &status.error {
        return match error {
            DevtoolStatusError::InvalidRecipeIdentity => {
                "invalid recipe identity; all actions disabled".into()
            }
            DevtoolStatusError::DevtoolFailed { exit_code, message } => format!(
                "status failed (exit {}): {message}; all actions disabled",
                exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
            ),
            DevtoolStatusError::MalformedOutput { line } => {
                format!("malformed Devtool output: {line}; all actions disabled")
            }
        };
    }
    let state = match &status.workspace {
        DevtoolWorkspace::NotMember => "not in workspace".into(),
        DevtoolWorkspace::MissingDirectory { source_path } => {
            format!("workspace source missing: {}", source_path.display())
        }
        DevtoolWorkspace::Present {
            source_path,
            recipe_file,
        } => {
            let git = match &status.git {
                DevtoolGitState::Available {
                    branch,
                    head,
                    modified,
                    untracked,
                    conflicted,
                } => format!(
                    "Git branch {}, head {}, {}, modified {modified}, untracked {untracked}, conflicted {conflicted}",
                    branch.as_deref().unwrap_or("detached"),
                    head.as_deref().unwrap_or("initial"),
                    if modified + untracked + conflicted == 0 {
                        "clean"
                    } else {
                        "dirty"
                    }
                ),
                DevtoolGitState::MissingExecutable => "Git executable missing".into(),
                DevtoolGitState::NotRepository => "source is not a Git repository".into(),
                DevtoolGitState::Failed { exit_code, message } => format!(
                    "Git status failed (exit {}): {message}",
                    exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
                ),
                DevtoolGitState::Malformed { message } => {
                    format!("malformed Git status: {message}")
                }
                DevtoolGitState::NotApplicable => "Git status not applicable".into(),
            };
            format!(
                "member at {}\nWorkspace recipe: {}\n{git}",
                source_path.display(),
                recipe_file
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string())
            )
        }
    };
    format!("{state}\n{}", devtool_action_status(status))
}

pub(crate) fn devtool_action_status(status: &DevtoolStatus) -> String {
    [
        ("d modify/edit", DevtoolAction::ModifyOrEdit),
        ("u update", DevtoolAction::UpdateRecipe),
        ("F finish", DevtoolAction::Finish),
        ("P deploy", DevtoolAction::Deploy),
        ("D reset", DevtoolAction::Reset),
    ]
    .into_iter()
    .map(|(label, action)| {
        status.disabled_reason(action).map_or_else(
            || format!("{label}: enabled"),
            |reason| format!("{label}: disabled ({reason})"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ")
}

pub(crate) fn recipe_values(label: &str, values: Option<&Vec<String>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values.join(", ")
            }
        },
    );
    format!("{label}: {value}")
}

pub(crate) fn recipe_paths(label: &str, values: Option<&Vec<std::path::PathBuf>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            }
        },
    );
    format!("{label}: {value}")
}

pub(crate) fn recipe_inspector(app: &App, recipe: &Recipe) -> String {
    let load_state = if app.recipe_metadata_loading.contains(&recipe.name) {
        "loading selected recipe metadata…".into()
    } else if let Some(error) = app.recipe_metadata_errors.get(&recipe.name) {
        format!("metadata unavailable: {error}")
    } else if app.recipe_metadata.contains_key(&recipe.name) {
        "metadata loaded".into()
    } else {
        "not loaded; press Enter to inspect".into()
    };
    let metadata = app.recipe_metadata.get(&recipe.name);
    let dependencies = app
        .dependencies
        .as_ref()
        .filter(|dependencies| dependencies.recipe == recipe.name);
    let active_tasks = app
        .tasks
        .values()
        .filter(|task| task.recipe == recipe.name)
        .map(|task| format!("{} ({:?})", task.task, task.state))
        .collect::<Vec<_>>();
    let reported_tasks = metadata.and_then(|metadata| metadata.tasks.as_ref());
    let standard_tasks = [
        "clean",
        "cleansstate",
        "devshell",
        "menuconfig",
        "diffconfig",
        "diffsigs",
    ];
    let (enabled, disabled): (Vec<_>, Vec<_>) = standard_tasks.into_iter().partition(|task| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    });
    let task_capabilities = if reported_tasks.is_none() {
        "Task actions unavailable until metadata is loaded.".into()
    } else {
        format!(
            "Task actions enabled: {}\nTask actions unavailable: {}",
            if enabled.is_empty() {
                "none".into()
            } else {
                enabled.join(", ")
            },
            if disabled.is_empty() {
                "none".into()
            } else {
                disabled.join(", ")
            }
        )
    };
    let retained_log_count = app
        .tasks
        .values()
        .chain(app.completed_tasks.iter().map(|completed| &completed.task))
        .filter(|task| task.recipe == recipe.name && task.log_path.is_some())
        .count();
    let patch_capability = match metadata.and_then(|value| value.patches.as_ref()) {
        None => "unavailable until metadata is loaded".into(),
        Some(patches) if patches.is_empty() => "unavailable (BitBake reported none)".into(),
        Some(patches) => {
            let local = patches
                .iter()
                .filter(|patch| std::path::Path::new(patch).is_absolute())
                .count();
            if local == 0 {
                "unavailable (remote or unresolved paths)".into()
            } else {
                format!("enabled ({local} local)")
            }
        }
    };
    let navigation_capabilities = format!(
        "Navigation: provider {}; task logs {}; patches {patch_capability}\nDevtool availability is authoritative above; press t to refresh.",
        if recipe.file.is_some() {
            "enabled"
        } else {
            "unavailable (provider path not reported)"
        },
        if retained_log_count == 0 {
            "unavailable (no retained path)".into()
        } else {
            format!("enabled ({retained_log_count})")
        },
    );
    let supports_task = |task: &str| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    };
    let qa_capabilities = if reported_tasks.is_none() {
        "QA actions unavailable until authoritative task metadata is loaded.".into()
    } else {
        format!(
            "QA actions: CVE check {}; SPDX generation {}.",
            if supports_task("cve_check") {
                "enabled"
            } else {
                "unavailable (do_cve_check not reported)"
            },
            if supports_task("create_spdx") {
                "enabled"
            } else {
                "unavailable (do_create_spdx not reported)"
            }
        )
    };
    let latest_qa_job = app.background_jobs.jobs.iter().rev().find(|job| {
        job.context.recipe.as_deref() == Some(recipe.name.as_str())
            && matches!(
                job.kind,
                BackgroundJobKind::CveCheck | BackgroundJobKind::Spdx
            )
    });
    let qa_job = latest_qa_job.map_or_else(
        || "Latest QA job: not run.".into(),
        |job| {
            let artifacts = if job
                .result
                .as_ref()
                .is_some_and(|result| !result.artifacts.is_empty())
            {
                job.result
                    .as_ref()
                    .unwrap()
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "none reported".into()
            };
            let detail = job
                .error
                .as_ref()
                .map(|error| error.summary.as_str())
                .or_else(|| job.result.as_ref().map(|result| result.summary.as_str()))
                .or_else(|| job.output.back().map(|entry| entry.message.as_str()))
                .unwrap_or("no retained result yet");
            format!(
                "Latest QA job: {} [{:?}], progress {:?}, warnings {}, errors {}.\nQA result: {detail}; artifacts: {artifacts}.",
                job.title, job.status, job.progress, job.warnings, job.errors
            )
        },
    );
    let devtool_job = app
        .background_jobs
        .jobs
        .iter()
        .rev()
        .find(|job| {
            job.kind == BackgroundJobKind::Devtool
                && job.context.recipe.as_deref() == Some(recipe.name.as_str())
        })
        .map_or_else(
            || "Latest Devtool job: not run.".into(),
            |job| {
                let output = job.output.back().map_or_else(
                    || "no retained output".into(),
                    |entry| {
                        format!(
                            "{:?}: {}{}",
                            entry.source,
                            entry.message,
                            if entry.truncated { " [truncated]" } else { "" }
                        )
                    },
                );
                let outcome = job
                    .result
                    .as_ref()
                    .map(|result| result.summary.as_str())
                    .or_else(|| job.error.as_ref().map(|error| error.summary.as_str()))
                    .unwrap_or("in progress");
                format!(
                    "Latest Devtool job: {} [{:?}].\nDevtool output: {output}; outcome: {outcome}.",
                    job.title, job.status
                )
            },
        );
    let tasks = if active_tasks.is_empty() {
        recipe_values("Tasks", reported_tasks)
    } else {
        format!("Active tasks: {}", active_tasks.join(", "))
    };
    format!(
        "Recipe: {}\nResolved version: {}\nPreferred version: {}\nProvider layer: {}\nProvider file: {}\nAppends: {}\nWorkspace/Devtool: {}\nBuild: {}\nDetail: {load_state}\n{task_capabilities}\n{navigation_capabilities}\n{qa_capabilities}\n{qa_job}\n{devtool_job}\n\nDependencies: {}\nRuntime dependencies: {}\nReverse dependencies: unavailable\n{tasks}\n{}\n{}\n{}\n{}",
        recipe.name,
        recipe.version.as_deref().unwrap_or("unavailable"),
        recipe.preferred_version.as_deref().unwrap_or("unavailable"),
        recipe.layer.as_deref().unwrap_or("unavailable"),
        recipe
            .file
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        recipe
            .append_count
            .map_or_else(|| "unavailable".into(), |count| count.to_string()),
        recipe_workspace_state(app, recipe),
        recipe_build_state(app, &recipe.name),
        dependencies.map_or_else(
            || "unavailable; press g to query".into(),
            |value| if value.build.is_empty() {
                "none".into()
            } else {
                value.build.join(", ")
            }
        ),
        dependencies.map_or_else(
            || "unavailable".into(),
            |value| if value.runtime.is_empty() {
                "none".into()
            } else {
                value.runtime.join(", ")
            }
        ),
        recipe_paths(
            "Metadata sources",
            metadata.and_then(|value| value.sources.as_ref())
        ),
        recipe_values("Patches", metadata.and_then(|value| value.patches.as_ref())),
        recipe_values(
            "Package outputs",
            metadata.and_then(|value| value.packages.as_ref())
        ),
        recipe_values("History", metadata.and_then(|value| value.history.as_ref())),
    )
}
