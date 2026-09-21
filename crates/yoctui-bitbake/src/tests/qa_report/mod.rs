use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::{QaLayerIdentity, QaScope, RecipeIdentity};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-qa-report-{}-{}",
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

fn recipe_scope(root: &Path) -> QaFindingScope {
    let provider = root.join("busybox.bb");
    fs::write(&provider, "SUMMARY = \"BusyBox\"\n").unwrap();
    QaFindingScope::Recipe(
        QaScope::new(RecipeIdentity {
            name: "busybox".into(),
            file: provider,
        })
        .unwrap(),
    )
}

fn layer_scope(root: &Path) -> QaFindingScope {
    let layer = root.join("meta-demo");
    fs::create_dir_all(&layer).unwrap();
    QaFindingScope::Layer(QaLayerIdentity::new("meta-demo".into(), layer).unwrap())
}

fn candidate(
    path: PathBuf,
    format: Option<QaReportFormat>,
    producer: QaCheckId,
    scope: QaFindingScope,
) -> QaReportCandidate {
    let (task, test_name) = match scope {
        QaFindingScope::Recipe(_) => (Some("do_package_qa".into()), None),
        QaFindingScope::Layer(_) => (None, Some("bsp.test".into())),
    };
    QaReportCandidate {
        path,
        origin: QaReportOrigin::Managed,
        format,
        producer,
        scope,
        task,
        test_name,
    }
}

fn input(root: &Path, candidates: Vec<QaReportCandidate>) -> QaReportScanInput {
    let mut paths = candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    let mut checks = candidates
        .iter()
        .map(|candidate| candidate.producer.clone())
        .collect::<Vec<_>>();
    checks.sort();
    checks.dedup();
    let mut scopes = candidates
        .iter()
        .map(|candidate| candidate.scope.clone())
        .collect::<Vec<_>>();
    scopes.sort_by_key(|scope| format!("{scope:?}"));
    scopes.dedup();
    QaReportScanInput {
        build_directory: root.into(),
        request: QaReportRequest::new(1, paths).unwrap(),
        candidates,
        known_checks: checks,
        known_scopes: scopes,
    }
}

mod qa_report_parses_exact_json_text_xml_and_bitbake_records;

mod qa_report_directory_scan_is_bounded_exact_and_partial;

mod qa_report_preserves_empty_and_malformed_as_distinct_outcomes;

mod qa_report_rejects_missing_symlink_escape_duplicate_and_mismatch;

mod qa_report_supports_imports_and_revalidates_stale_identity;

mod qa_report_preserves_cancellation_timeout_loss_and_oversize;
