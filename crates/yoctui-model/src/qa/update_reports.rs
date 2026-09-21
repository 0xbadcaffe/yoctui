fn update_reports(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::RefreshReports => {
            let Some(request) = state.inventory.request().cloned() else {
                return QaTransition::notify("Import or run a QA check first.");
            };
            match begin_report_request(state, request.paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::ReportsLoaded {
            request,
            reports,
            limitations,
        } if exact_request_matches(state, &request) => {
            let known_checks = state
                .capability
                .snapshot()
                .map(|snapshot| {
                    snapshot
                        .checks
                        .iter()
                        .map(|check| check.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut known_checks = known_checks;
            let mut known_scopes = state
                .capability
                .snapshot()
                .map(|snapshot| {
                    snapshot
                        .scopes
                        .iter()
                        .cloned()
                        .map(QaFindingScope::Recipe)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some(snapshot) = state.layer_capability.snapshot() {
                known_checks.extend(snapshot.layers.iter().map(|layer| layer.check.clone()));
                known_checks.sort();
                known_checks.dedup();
                known_scopes.extend(
                    snapshot
                        .layers
                        .iter()
                        .map(|layer| QaFindingScope::Layer(layer.identity.clone())),
                );
                known_scopes.sort_by(|left, right| {
                    left.name()
                        .cmp(right.name())
                        .then_with(|| left.path().cmp(right.path()))
                });
                known_scopes.dedup();
            }
            let (reports, mut model_limitations) =
                normalize_qa_reports(reports, &known_checks, &known_scopes);
            model_limitations.extend(limitations);
            let limitations = normalize_limitations(model_limitations);
            state.inventory = if reports.is_empty() && limitations.is_empty() {
                QaReportInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                QaReportInventoryState::Available { request, reports }
            } else {
                QaReportInventoryState::Partial {
                    request,
                    reports,
                    limitations,
                }
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsFailed {
            request,
            kind,
            message,
        } if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Failed {
                request,
                kind,
                message,
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsCancelled(request) if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Cancelled { request };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsTimedOut(request) if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::TimedOut { request };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsLost { request, message } if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Lost { request, message };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsLoaded { .. }
        | QaAction::ReportsFailed { .. }
        | QaAction::ReportsCancelled(_)
        | QaAction::ReportsTimedOut(_)
        | QaAction::ReportsLost { .. } => QaTransition::none(),
        _ => QaTransition::none(),
    }
}
