fn update_inspection(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::CycleView { backwards } => {
            state.view = state.view.cycle(backwards);
        }
        MaintenanceAction::Select { delta, row_count } => {
            let selection = &mut state.selections[state.view.index()];
            *selection = if delta.is_negative() {
                selection.saturating_sub(delta.unsigned_abs())
            } else {
                selection
                    .saturating_add(delta as usize)
                    .min(row_count.saturating_sub(1))
            };
        }
        MaintenanceAction::InspectCapability => {
            let request = next_id(&mut state.capability_generation);
            state.capability = MaintenanceCapability::Loading(request);
            state.services = MaintenanceServiceDiagnostics::Loading(request);
            state.integrations = MaintenanceIntegrationDiagnostics::Loading(request);
            return MaintenanceTransition::effect(MaintenanceEffect::InspectCapability { request });
        }
        MaintenanceAction::CapabilityLoaded {
            request,
            snapshot,
            partial,
        } if state.capability.request() == Some(request) => {
            state.capability = if partial {
                MaintenanceCapability::Partial {
                    request,
                    limitations: snapshot.limitations.clone(),
                    snapshot,
                }
            } else {
                MaintenanceCapability::Available { request, snapshot }
            };
        }
        MaintenanceAction::CapabilityFailed { request, message }
            if state.capability.request() == Some(request) && bounded_text(&message) =>
        {
            state.capability = MaintenanceCapability::Failed { request, message };
        }
        MaintenanceAction::IntegrationsLoaded {
            request,
            snapshot,
            partial,
        } if state.integrations.request() == Some(request) => {
            let snapshot = *snapshot;
            state.integrations = if partial {
                MaintenanceIntegrationDiagnostics::Partial {
                    request,
                    limitations: snapshot.limitations.clone(),
                    snapshot,
                }
            } else {
                MaintenanceIntegrationDiagnostics::Available { request, snapshot }
            };
        }
        MaintenanceAction::IntegrationsFailed { request, message }
            if state.integrations.request() == Some(request) && bounded_text(&message) =>
        {
            state.integrations = MaintenanceIntegrationDiagnostics::Failed { request, message };
        }
        MaintenanceAction::InspectServices => {
            let request = next_id(&mut state.service_generation);
            state.services = MaintenanceServiceDiagnostics::Loading(request);
            return MaintenanceTransition::effect(MaintenanceEffect::InspectServices { request });
        }
        MaintenanceAction::ServicesLoaded {
            request,
            mut services,
            limitations,
        } if state.services.request() == Some(request) => {
            services.truncate(3);
            let limitations = normalize_text(limitations, MAX_MAINTENANCE_LIMITATIONS);
            state.services = if limitations.is_empty() {
                MaintenanceServiceDiagnostics::Available { request, services }
            } else {
                MaintenanceServiceDiagnostics::Partial {
                    request,
                    services,
                    limitations,
                }
            };
        }
        MaintenanceAction::ServicesFailed { request, message }
            if state.services.request() == Some(request) && bounded_text(&message) =>
        {
            state.services = MaintenanceServiceDiagnostics::Failed { request, message };
        }
        _ => {}
    }
    MaintenanceTransition::none()
}
