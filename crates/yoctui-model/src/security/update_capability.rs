fn update_capability(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::InspectCapability => {
            state.capability = SecurityCapability::Inspecting;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::CapabilityLoaded(capability) => {
            state.scope = Some(capability.scope.clone());
            state.capability = SecurityCapability::Available(Box::new(capability));
            SecurityTransition::none()
        }
        SecurityAction::CapabilityFailed(message) => {
            state.capability = SecurityCapability::Failed(message);
            SecurityTransition::none()
        }
        SecurityAction::CycleView => {
            state.view = match state.view {
                SecurityView::Cves => SecurityView::Sbom,
                SecurityView::Sbom => SecurityView::Cves,
            };
            state.drilled = false;
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::CycleScope => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let current = capability
                .available_scopes
                .iter()
                .position(|scope| Some(scope) == state.scope.as_ref())
                .unwrap_or(0);
            let Some(scope) = capability
                .available_scopes
                .get((current + 1) % capability.available_scopes.len().max(1))
                .cloned()
            else {
                return SecurityTransition::notify("No alternate Security scope is available.");
            };
            state.scope = Some(scope);
            state.capability = SecurityCapability::NotInspected;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::SetScope(scope) if scope.is_valid() => {
            state.scope = Some(scope);
            state.capability = SecurityCapability::NotInspected;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::SetScope(_) => SecurityTransition::notify("Invalid Security scope."),
        SecurityAction::BeginCveCheck => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let Some(task) = capability.cve_task.clone() else {
                return SecurityTransition::notify(
                    capability
                        .cve_unavailable_reason()
                        .unwrap_or("CVE check is unavailable."),
                );
            };
            let request = BuildRequest {
                targets: vec![capability.scope.target().into()],
                task: Some(task),
                force: false,
            };
            match operation_preview(
                state,
                SecurityOperation::CveCheck(request),
                capability.cve_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::BeginSbomGeneration => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let task = match &capability.scope {
                SecurityScope::Recipe(_) => capability.recipe_sbom_task.clone(),
                SecurityScope::Image { .. } => capability.image_sbom_task.clone(),
            };
            if task.is_none() && !capability.image_build_emits_sbom {
                return SecurityTransition::notify(
                    capability
                        .sbom_unavailable_reason()
                        .unwrap_or("SBOM generation is unavailable."),
                );
            }
            let request = BuildRequest {
                targets: vec![capability.scope.target().into()],
                task,
                force: false,
            };
            match operation_preview(
                state,
                SecurityOperation::SbomBuild(request),
                capability.sbom_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::BeginPackageMap => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let Some(mapper) = capability.mapper.clone() else {
                return SecurityTransition::notify("cve-check-map-pkgs is unavailable.");
            };
            match operation_preview(
                state,
                SecurityOperation::PackageMap {
                    executable: mapper.executable,
                    arguments: mapper.arguments,
                },
                capability.cve_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::ConfirmOperation(preview)
            if state.active_session().is_none()
                && preview.id.0 != 0
                && Some(&preview.scope) == state.scope.as_ref() =>
        {
            let effect = match &preview.operation {
                SecurityOperation::CveCheck(request) | SecurityOperation::SbomBuild(request) => {
                    SecurityEffect::StartBuild {
                        id: preview.id,
                        request: request.clone(),
                    }
                }
                SecurityOperation::PackageMap {
                    executable,
                    arguments,
                } => SecurityEffect::StartPackageMap {
                    id: preview.id,
                    executable: executable.clone(),
                    arguments: arguments.clone(),
                },
            };
            state.sessions.push(SecuritySession {
                preview,
                status: SecuritySessionStatus::Starting,
                background_job_id: None,
                started_at: SystemTime::now(),
                finished_at: None,
                message: None,
                result_paths: Vec::new(),
                output: Vec::new(),
            });
            if state.sessions.len() > MAX_SECURITY_SESSIONS {
                state.sessions.remove(0);
            }
            SecurityTransition {
                effect: Some(effect),
                dialog: SecurityDialogUpdate::Close,
                notification: None,
            }
        }
        SecurityAction::ConfirmOperation(_) => {
            SecurityTransition::notify("The Security operation preview is stale.")
        }
        _ => SecurityTransition::none(),
    }
}
