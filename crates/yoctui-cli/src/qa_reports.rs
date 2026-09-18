//! Qa reports.
use super::*;

pub(crate) fn qa_report_scan_input(
    app: &App,
    build_directory: PathBuf,
    request: QaReportRequest,
) -> std::result::Result<QaReportScanInput, String> {
    let known_checks = app
        .qa
        .capability
        .snapshot()
        .into_iter()
        .flat_map(|snapshot| snapshot.checks.iter().map(|check| check.id.clone()))
        .chain(
            app.qa
                .layer_capability
                .snapshot()
                .into_iter()
                .flat_map(|snapshot| snapshot.layers.iter().map(|layer| layer.check.clone())),
        )
        .collect::<Vec<_>>();
    let known_scopes = app
        .qa
        .capability
        .snapshot()
        .into_iter()
        .flat_map(|snapshot| snapshot.scopes.iter().cloned().map(QaFindingScope::Recipe))
        .chain(
            app.qa
                .layer_capability
                .snapshot()
                .into_iter()
                .flat_map(|snapshot| {
                    snapshot
                        .layers
                        .iter()
                        .map(|layer| QaFindingScope::Layer(layer.identity.clone()))
                }),
        )
        .collect::<Vec<_>>();
    let candidates = request
        .paths
        .iter()
        .map(|path| qa_report_candidate(app, path))
        .collect::<std::result::Result<Vec<_>, String>>()?;
    Ok(QaReportScanInput {
        build_directory,
        request,
        candidates,
        known_checks,
        known_scopes,
    })
}

pub(crate) fn qa_report_candidate(
    app: &App,
    path: &Path,
) -> std::result::Result<QaReportCandidate, String> {
    let recipe_session = app
        .qa
        .sessions
        .iter()
        .rev()
        .find(|session| {
            session
                .result_paths
                .iter()
                .any(|candidate| candidate == path)
        })
        .map(|session| {
            (
                QaReportOrigin::Managed,
                session.operation.check.clone(),
                QaFindingScope::Recipe(session.operation.scope.clone()),
                session.operation.request.task.clone(),
                None,
            )
        });
    let layer_session = app
        .qa
        .layer_sessions
        .iter()
        .rev()
        .find(|session| {
            session
                .result_paths
                .iter()
                .any(|candidate| candidate == path)
        })
        .map(|session| {
            (
                QaReportOrigin::Managed,
                session.operation.check.clone(),
                QaFindingScope::Layer(session.operation.layer.clone()),
                None,
                Some("yocto-check-layer".into()),
            )
        });
    let retained_report = app
        .qa
        .inventory
        .reports()
        .unwrap_or_default()
        .iter()
        .find(|report| report.identity.path == path)
        .and_then(|report| {
            let producer = report.identity.producer.clone()?;
            let scope = report.identity.scope.clone()?;
            let (task, test_name) = match &scope {
                QaFindingScope::Recipe(recipe_scope) => {
                    let task = app.qa.capability.snapshot().and_then(|snapshot| {
                        snapshot
                            .checks
                            .iter()
                            .find(|check| check.id == producer && check.scope == *recipe_scope)
                            .and_then(|check| check.task.clone())
                    });
                    (task, None)
                }
                QaFindingScope::Layer(_) => (None, Some("yocto-check-layer".into())),
            };
            Some((QaReportOrigin::Import, producer, scope, task, test_name))
        });
    let selected_recipe = app.qa.selected_check().map(|check| {
        (
            QaReportOrigin::Import,
            check.id.clone(),
            QaFindingScope::Recipe(check.scope.clone()),
            check.task.clone(),
            None,
        )
    });
    let selected_layer = app.qa.selected_layer().map(|layer| {
        (
            QaReportOrigin::Import,
            layer.check.clone(),
            QaFindingScope::Layer(layer.identity.clone()),
            None,
            Some("yocto-check-layer".into()),
        )
    });
    let (origin, producer, scope, task, test_name) = recipe_session
        .or(layer_session)
        .or(retained_report)
        .or_else(|| match app.qa.view {
            yoctui_model::QaView::RecipeKernel => selected_recipe.or(selected_layer),
            yoctui_model::QaView::LayerQa => selected_layer.or(selected_recipe),
        })
        .ok_or_else(|| {
            "QA report import needs an exact check or configured-layer scope".to_owned()
        })?;
    let format = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => None,
        _ => qa_report_format(path),
    };
    Ok(QaReportCandidate {
        path: path.to_path_buf(),
        origin,
        format,
        producer,
        scope,
        task,
        test_name,
    })
}

pub(crate) fn qa_report_format(path: &Path) -> Option<QaReportFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json" | "jsonl") => Some(QaReportFormat::Json),
        Some("xml") => Some(QaReportFormat::Xml),
        Some("qa" | "txt") => Some(QaReportFormat::Text),
        Some("log") => Some(QaReportFormat::BitBakeLog),
        _ => None,
    }
}
