fn update_navigation(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::SelectReport(delta) => {
            let identities = state
                .visible_reports()
                .into_iter()
                .map(|report| report.identity().clone())
                .collect::<Vec<_>>();
            let current = state
                .report_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.report_selection = identities.get(next).cloned();
            state.drilled = false;
            SecurityTransition::none()
        }
        SecurityAction::SelectFinding(delta) => {
            let identities = state
                .visible_findings()
                .into_iter()
                .map(|finding| finding.identity.clone())
                .collect::<Vec<_>>();
            let current = state
                .finding_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.finding_selection = identities.get(next).cloned();
            SecurityTransition::none()
        }
        SecurityAction::SelectComponent(delta) => {
            let identities = state
                .visible_components()
                .into_iter()
                .map(|component| component.identity.clone())
                .collect::<Vec<_>>();
            let current = state
                .component_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.component_selection = identities.get(next).cloned();
            SecurityTransition::none()
        }
        SecurityAction::Drill => {
            if matches!(
                state.selected_report(),
                Some(
                    SecurityReport::Spdx(_)
                        | SecurityReport::CycloneDx(_)
                        | SecurityReport::PackageManifest(_)
                )
            ) {
                state.drilled = true;
                state.component_selection = state
                    .visible_components()
                    .first()
                    .map(|component| component.identity.clone());
            }
            SecurityTransition::none()
        }
        SecurityAction::LeaveDrill => {
            state.drilled = false;
            SecurityTransition::none()
        }
        SecurityAction::BeginSearch => {
            state.searching = true;
            SecurityTransition::none()
        }
        SecurityAction::AppendQuery(character)
            if state.searching
                && !character.is_control()
                && state.query.len() + character.len_utf8() <= MAX_SECURITY_QUERY_BYTES =>
        {
            state.query.push(character);
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::BackspaceQuery if state.searching => {
            state.query.pop();
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::ClearQuery => {
            state.query.clear();
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::FinishSearch => {
            state.searching = false;
            SecurityTransition::none()
        }
        SecurityAction::AppendQuery(_) | SecurityAction::BackspaceQuery => {
            SecurityTransition::none()
        }
        SecurityAction::CycleCveFilter => {
            state.cve_filter = state.cve_filter.next();
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::OpenSelectedReport => state.selected_report().map_or_else(
            || SecurityTransition::notify("Select an exact Security report first."),
            |report| {
                SecurityTransition::effect(SecurityEffect::OpenPath(report.identity().path.clone()))
            },
        ),
        SecurityAction::OpenSelectedRecipe => selected_provider(state).map_or_else(
            || SecurityTransition::notify("No exact recipe provider is available."),
            |path| SecurityTransition::effect(SecurityEffect::OpenPath(path)),
        ),
        SecurityAction::OpenSelectedAdvisory => {
            let finding = state
                .visible_findings()
                .into_iter()
                .find(|finding| Some(&finding.identity) == state.finding_selection.as_ref());
            finding
                .and_then(|finding| finding.advisory_url.clone())
                .map_or_else(
                    || SecurityTransition::notify("No exact HTTPS advisory URL is available."),
                    |url| SecurityTransition::effect(SecurityEffect::OpenUrl(url)),
                )
        }        _ => SecurityTransition::none(),
    }
}
