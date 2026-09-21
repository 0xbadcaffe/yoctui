fn update_reports(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::RefreshReports => {
            let Some(request) = state.inventory.request().cloned() else {
                return SecurityTransition::notify("Import or discover Security reports first.");
            };
            match begin_report_request(state, request.paths) {
                Ok(effect) => SecurityTransition::effect(effect),
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::ReportsLoaded {
            request,
            reports,
            limitations,
        } if exact_request_matches(state, &request) => {
            let (reports, mut model_limitations) = normalize_security_reports(reports);
            model_limitations.extend(limitations);
            let limitations = normalize_limitations(model_limitations);
            state.inventory = if reports.is_empty() && limitations.is_empty() {
                SecurityInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                SecurityInventoryState::Available { request, reports }
            } else {
                SecurityInventoryState::Partial {
                    request,
                    reports,
                    limitations,
                }
            };
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::ReportsFailed { request, message }
            if exact_request_matches(state, &request) =>
        {
            state.inventory = SecurityInventoryState::Failed { request, message };
            SecurityTransition::none()
        }
        SecurityAction::ReportsCancelled(request) if exact_request_matches(state, &request) => {
            state.inventory = SecurityInventoryState::Cancelled { request };
            SecurityTransition::none()
        }
        SecurityAction::ReportsTimedOut(request) if exact_request_matches(state, &request) => {
            state.inventory = SecurityInventoryState::TimedOut { request };
            SecurityTransition::none()
        }
        SecurityAction::ReportsLost { request, message }
            if exact_request_matches(state, &request) =>
        {
            state.inventory = SecurityInventoryState::Lost { request, message };
            SecurityTransition::none()
        }
        SecurityAction::ReportsLoaded { .. }
        | SecurityAction::ReportsFailed { .. }
        | SecurityAction::ReportsCancelled(_)
        | SecurityAction::ReportsTimedOut(_)
        | SecurityAction::ReportsLost { .. } => SecurityTransition::none(),
        _ => SecurityTransition::none(),
    }
}
