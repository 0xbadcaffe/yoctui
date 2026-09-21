fn update_report_navigation(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::SelectReport(delta) => {
            let reports = state
                .inventory
                .reports()
                .unwrap_or_default()
                .iter()
                .map(|report| report.identity.clone())
                .collect::<Vec<_>>();
            state.report_selection = select_index(&reports, state.report_selection.as_ref(), delta);
            QaTransition::none()
        }
        QaAction::SelectFinding(delta) => {
            let findings = state
                .visible_findings()
                .into_iter()
                .map(|finding| finding.identity.clone())
                .collect::<Vec<_>>();
            state.finding_selection =
                select_index(&findings, state.finding_selection.as_ref(), delta);
            QaTransition::none()
        }
        QaAction::Drill => {
            if (state.view == QaView::RecipeKernel && state.selected_check().is_some())
                || (state.view == QaView::LayerQa && state.selected_layer().is_some())
            {
                state.drilled = true;
                clamp_selection(state);
            }
            QaTransition::none()
        }
        QaAction::LeaveDrill => {
            state.drilled = false;
            QaTransition::none()
        }
        QaAction::BeginSearch => {
            state.searching = true;
            QaTransition::none()
        }
        QaAction::AppendQuery(character)
            if state.searching
                && !character.is_control()
                && state.query.len() + character.len_utf8() <= MAX_QA_QUERY_BYTES =>
        {
            state.query.push(character);
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BackspaceQuery if state.searching => {
            state.query.pop();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ClearQuery => {
            state.query.clear();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::FinishSearch => {
            state.searching = false;
            QaTransition::none()
        }
        QaAction::AppendQuery(_) | QaAction::BackspaceQuery => QaTransition::none(),
        QaAction::CycleStatusFilter => {
            state.status_filter = state.status_filter.next();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::OpenSelectedReport => state.selected_report().map_or_else(
            || QaTransition::notify("Select an exact QA report first."),
            |report| QaTransition::effect(QaEffect::OpenReport(report.identity.clone())),
        ),
        QaAction::OpenProvider => state.scope.as_ref().map_or_else(
            || QaTransition::notify("No exact QA provider is selected."),
            |scope| QaTransition::effect(QaEffect::OpenProvider(scope.recipe.clone())),
        ),
        QaAction::OpenSelectedSource => state
            .selected_finding()
            .and_then(|finding| finding.source.clone())
            .map_or_else(
                || QaTransition::notify("No exact QA finding source is available."),
                |source| QaTransition::effect(QaEffect::OpenSource(source)),
            ),
        _ => QaTransition::none(),
    }
}
