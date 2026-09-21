fn update_layer_capability(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::InspectLayerCapability => {
            state.layer_capability = QaLayerCapability::Inspecting;
            QaTransition::effect(QaEffect::InspectLayerCapability)
        }
        QaAction::LayerCapabilityLoaded(snapshot) => {
            if !snapshot.is_valid() {
                state.layer_capability =
                    QaLayerCapability::Failed("Layer QA capability response is invalid.".into());
                state.layer_selection = None;
                return QaTransition::notify("Layer QA capability response is invalid.");
            }
            state.layer_selection = Some(snapshot.selected_layer.clone());
            state.layer_capability = QaLayerCapability::Available(Box::new(snapshot));
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::LayerCapabilityPartial {
            snapshot,
            limitations,
        } => {
            if !snapshot.is_valid() {
                state.layer_capability =
                    QaLayerCapability::Failed("Layer QA capability response is invalid.".into());
                state.layer_selection = None;
                return QaTransition::notify("Layer QA capability response is invalid.");
            }
            state.layer_selection = Some(snapshot.selected_layer.clone());
            state.layer_capability = QaLayerCapability::Partial {
                snapshot: Box::new(snapshot),
                limitations: normalize_limitations(limitations),
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::LayerCapabilityFailed(message) => {
            state.layer_capability = QaLayerCapability::Failed(message);
            state.layer_selection = None;
            QaTransition::none()
        }
        QaAction::SelectLayer(delta) => {
            let layers = state
                .visible_layers()
                .into_iter()
                .map(|layer| layer.identity.clone())
                .collect::<Vec<_>>();
            state.layer_selection = select_index(&layers, state.layer_selection.as_ref(), delta);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BeginSelectedLayerCheck => {
            if state.active_layer_session().is_some() {
                return QaTransition::notify("A layer QA operation is already active.");
            }
            let Some(capability) = state.selected_layer().cloned() else {
                return QaTransition::notify("Select an exact configured layer first.");
            };
            let (executable, arguments, report_roots) = match capability.run {
                QaLayerRunCapability::Available {
                    executable,
                    arguments,
                    report_roots,
                } => (executable, arguments, report_roots),
                QaLayerRunCapability::Disabled(reason) => {
                    return QaTransition::notify(reason);
                }
            };
            let preview = QaLayerOperationPreview {
                id: QaLayerOperationId(next_id(&mut state.layer_operation_generation)),
                check: capability.check,
                layer: capability.identity,
                indexed_arguments: indexed_native_arguments(&executable, &arguments),
                executable,
                arguments,
                report_roots,
                limitations: capability.limitations,
            };
            state.pending_layer_operation = Some(preview.clone());
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::LayerOperation(preview))),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmLayerOperation(preview) => {
            if state.pending_layer_operation.as_ref() != Some(&preview)
                || exact_layer_capability(state, &preview).is_none()
                || state.active_layer_session().is_some()
            {
                return QaTransition::notify(
                    "The layer QA confirmation is stale or no longer available.",
                );
            }
            let session = QaLayerSessionId(next_id(&mut state.layer_session_generation));
            state.layer_sessions.push_back(QaLayerSession {
                id: session,
                operation: preview.clone(),
                status: QaSessionStatus::Starting,
                started_at: SystemTime::now(),
                finished_at: None,
                exit_code: None,
                message: None,
                result_paths: Vec::new(),
                output: VecDeque::new(),
                dropped_output: 0,
            });
            while state.layer_sessions.len() > MAX_QA_SESSIONS {
                state.layer_sessions.pop_front();
            }
            state.pending_layer_operation = None;
            QaTransition {
                effect: Some(QaEffect::StartLayerCheck {
                    session,
                    layer: preview.layer,
                    executable: preview.executable,
                    arguments: preview.arguments,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        _ => QaTransition::none(),
    }
}
