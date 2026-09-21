use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-qa-layer-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

#[cfg(unix)]
fn fixture(name: &str, body: &str) -> (TestDirectory, QaLayerCapabilitySnapshot) {
    let root = TestDirectory::new(name);
    let bin = root.0.join("bin");
    let layer = root.0.join("meta-demo");
    let reports = root.0.join("reports");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&layer).unwrap();
    fs::create_dir(&reports).unwrap();
    write_executable(&bin.join("yocto-check-layer"), body);
    let identity = QaLayerIdentity::new("meta-demo".into(), layer).unwrap();
    let input = QaLayerCapabilityInput {
        release: Some("6.0".into()),
        build_directory: root.0.clone(),
        selected_layer: identity.clone(),
        layers: vec![QaConfiguredLayerInput {
            check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
            identity,
            compatible_series: vec!["walnascar".into()],
            report_roots: vec![reports],
        }],
        executable_search_path: vec![bin],
    };
    let response = QaLayerCapabilityInspector::inspect(input).unwrap();
    let snapshot = match response {
        QaLayerCapabilityResponse::Available(snapshot) => snapshot,
        QaLayerCapabilityResponse::Partial(_) => panic!("expected complete capability"),
    };
    (root, snapshot)
}

fn preview(snapshot: &QaLayerCapabilitySnapshot) -> QaLayerOperationPreview {
    let layer = &snapshot.layers[0];
    let QaLayerRunCapability::Available {
        executable,
        arguments,
        report_roots,
    } = &layer.run
    else {
        panic!("expected runnable layer");
    };
    QaLayerOperationPreview {
        id: yoctui_model::QaLayerOperationId(4),
        check: layer.check.clone(),
        layer: layer.identity.clone(),
        executable: executable.clone(),
        arguments: arguments.clone(),
        indexed_arguments: indexed_arguments(&executable.path, arguments),
        report_roots: report_roots.clone(),
        limitations: Vec::new(),
    }
}

#[cfg(unix)]
mod qa_layer_capability_discovers_exact_configured_layers_and_partial_inputs;

#[cfg(unix)]
mod qa_layer_capability_and_command_reject_symlink_tampering_and_preview_changes;

#[cfg(unix)]
mod qa_layer_runner_streams_bounded_output_and_reports_success_and_nonzero;

#[cfg(unix)]
mod qa_layer_runner_rejects_duplicate_and_cancels_gracefully_or_forcibly;

#[cfg(unix)]
mod qa_layer_spawn_retry_classifies_only_text_file_busy_as_transient;

#[cfg(unix)]
mod qa_layer_runner_preserves_timeout_rejection_and_channel_loss;
