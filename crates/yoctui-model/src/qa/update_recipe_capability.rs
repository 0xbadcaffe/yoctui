fn update_recipe_capability(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::CycleView => {
            state.view = match state.view {
                QaView::RecipeKernel => QaView::LayerQa,
                QaView::LayerQa => QaView::RecipeKernel,
            };
            state.drilled = false;
            clamp_selection(state);
            if state.view == QaView::LayerQa
                && matches!(state.layer_capability, QaLayerCapability::NotInspected)
            {
                state.layer_capability = QaLayerCapability::Inspecting;
                return QaTransition::effect(QaEffect::InspectLayerCapability);
            }
            QaTransition::none()
        }
        QaAction::InspectCapability => {
            state.capability = QaCapability::Inspecting;
            QaTransition::effect(QaEffect::InspectCapability {
                scope: state.scope.clone(),
            })
        }
        QaAction::CapabilityLoaded(snapshot) => {
            if !snapshot.is_valid() {
                state.capability =
                    QaCapability::Failed("QA capability response is invalid.".into());
                state.check_selection = None;
                return QaTransition::notify("QA capability response is invalid.");
            }
            state.scope = Some(snapshot.selected_scope.clone());
            state.capability = QaCapability::Available(Box::new(snapshot));
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::CapabilityPartial {
            snapshot,
            limitations,
        } => {
            if !snapshot.is_valid() {
                state.capability =
                    QaCapability::Failed("QA capability response is invalid.".into());
                state.check_selection = None;
                return QaTransition::notify("QA capability response is invalid.");
            }
            state.scope = Some(snapshot.selected_scope.clone());
            state.capability = QaCapability::Partial {
                snapshot: Box::new(snapshot),
                limitations: normalize_limitations(limitations),
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::CapabilityFailed(message) => {
            state.capability = QaCapability::Failed(message);
            state.check_selection = None;
            QaTransition::none()
        }
        QaAction::CycleScope => {
            let Some(snapshot) = state.capability.snapshot() else {
                return QaTransition::notify("QA capability is not available.");
            };
            let current = snapshot
                .scopes
                .iter()
                .position(|scope| Some(scope) == state.scope.as_ref())
                .unwrap_or(0);
            let Some(scope) = snapshot
                .scopes
                .get((current + 1) % snapshot.scopes.len().max(1))
                .cloned()
            else {
                return QaTransition::notify("No exact QA recipe scope is available.");
            };
            state.scope = Some(scope);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::SelectCheck(delta) => {
            let checks = state
                .visible_checks()
                .into_iter()
                .map(|check| check.id.clone())
                .collect::<Vec<_>>();
            state.check_selection = select_index(&checks, state.check_selection.as_ref(), delta);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BeginSelectedCheck => {
            if state.active_session().is_some() {
                return QaTransition::notify("A QA operation is already active.");
            }
            let Some(check) = state.selected_check().cloned() else {
                return QaTransition::notify("Select an exact QA check first.");
            };
            let Some(task) = check.task.clone() else {
                return QaTransition::notify(
                    check
                        .availability
                        .disabled_reason()
                        .unwrap_or("The selected QA check is unavailable."),
                );
            };
            if !matches!(check.availability, QaCheckAvailability::Available) {
                return QaTransition::notify(
                    check
                        .availability
                        .disabled_reason()
                        .unwrap_or("The selected QA check is unavailable."),
                );
            }
            let request = BuildRequest {
                targets: vec![check.scope.recipe.name.clone()],
                task: Some(task),
                force: false,
            };
            if request.validate().is_err() {
                return QaTransition::notify("The capability supplied an invalid BitBake request.");
            }
            let preview = QaOperationPreview {
                id: QaOperationId(next_id(&mut state.operation_generation)),
                check: check.id,
                family: check.family,
                scope: check.scope,
                indexed_arguments: indexed_build_arguments(&request),
                request,
                report_roots: check.report_roots,
                limitations: check.limitations,
            };
            state.pending_operation = Some(preview.clone());
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::Operation(preview))),
                ..QaTransition::none()
            }
        }
        _ => QaTransition::none(),
    }
}
