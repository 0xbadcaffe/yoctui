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

mod qa_task_capability_uses_only_exact_family_bindings_and_reported_tasks;

mod qa_task_capability_accepts_alternate_tasks_only_as_explicit_data;

mod qa_task_capability_never_guesses_missing_or_similar_tasks;

mod qa_task_capability_preserves_usable_inputs_as_partial;

mod qa_task_capability_rejects_stale_selected_provider_and_symlinks;

mod qa_task_capability_handles_duplicates_ambiguity_and_bounds;
