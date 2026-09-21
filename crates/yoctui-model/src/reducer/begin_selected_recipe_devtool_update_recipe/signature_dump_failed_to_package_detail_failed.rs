use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::SignatureDumpFailed { target, message } => {
            if !matches!(
                &app.signature_dump,
                SignatureDumpState::Loading { target: requested } if requested == &target
            ) {
                return None;
            }
            app.signature_dump = SignatureDumpState::Failed {
                target,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature dump is unavailable: {message}"));
        }
        Action::SelectSignatureRecord { delta } => {
            let records = app.signature_dump.records()?;
            if records.is_empty() {
                app.signature_selection = None;
                return None;
            }
            let current = app
                .signature_selection
                .as_ref()
                .and_then(|selected| {
                    records
                        .iter()
                        .position(|record| &record.identity == selected)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(records.len().saturating_sub(1))
            };
            app.signature_selection = Some(records[next].identity.clone());
        }
        Action::SetSelectedSignatureComparisonSide(side) => {
            let Some(selected) = app.signature_selection.clone() else {
                app.notification = Some("No signature record is selected.".into());
                return None;
            };
            if !app
                .signature_dump
                .records()
                .is_some_and(|records| records.iter().any(|record| record.identity == selected))
            {
                app.notification =
                    Some("The selected signature is not in the current dump result.".into());
                return None;
            }
            let (mut left, mut right) = signature_comparison_inputs(&app.signature_comparison);
            match side {
                SignatureComparisonSide::Left => left = Some(selected),
                SignatureComparisonSide::Right => right = Some(selected),
            }
            app.signature_comparison = SignatureComparisonState::Ready { left, right };
        }
        Action::BeginSignatureComparison => {
            let (Some(left), Some(right)) = signature_comparison_inputs(&app.signature_comparison)
            else {
                app.notification =
                    Some("Select both left and right signature records before comparing.".into());
                return None;
            };
            let request = SignatureComparisonRequest { left, right };
            if let Err(message) = request.validate() {
                app.notification = Some(message.into());
                return None;
            }
            if !app.signature_dump.records().is_some_and(|records| {
                records.iter().any(|record| record.identity == request.left)
                    && records
                        .iter()
                        .any(|record| record.identity == request.right)
            }) {
                app.notification = Some(
                    "Both signature comparison inputs must be in the current dump result.".into(),
                );
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Loading {
                request: request.clone(),
            };
            return Some(Effect::CompareSignatures(request));
        }
        Action::SignatureComparisonLoaded {
            request,
            differences,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            app.signature_comparison = if report.is_partial() {
                SignatureComparisonState::Partial {
                    request,
                    differences,
                    limitations: vec![format!(
                        "Model bounds truncated {} signature differences.",
                        report.truncated_differences
                    )],
                }
            } else if differences.is_empty() {
                SignatureComparisonState::AvailableEmpty { request }
            } else {
                SignatureComparisonState::Available {
                    request,
                    differences,
                }
            };
        }
        Action::SignatureComparisonPartial {
            request,
            differences,
            mut limitations,
        } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            let (differences, report) =
                normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
            if report.is_partial() {
                limitations.push(format!(
                    "Model bounds truncated {} signature differences.",
                    report.truncated_differences
                ));
            }
            app.signature_comparison = SignatureComparisonState::Partial {
                request,
                differences,
                limitations,
            };
        }
        Action::SignatureComparisonFailed { request, message } => {
            if !matches!(
                &app.signature_comparison,
                SignatureComparisonState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            app.signature_comparison = SignatureComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Signature comparison failed: {message}"));
        }
        Action::BeginPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::RefreshPackageInventory => {
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_inventory(app));
        }
        Action::CancelPackageOperation => {
            if package_operation_is_loading(app) {
                return Some(Effect::CancelPackageOperation);
            }
            app.notification = Some("No package-data operation is running.".into());
        }
        Action::PackageInventoryLoaded { request, packages } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, None);
        }
        Action::PackageInventoryPartial {
            request,
            packages,
            limitations,
        } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            set_package_inventory(app, request, packages, Some(limitations));
        }
        Action::PackageInventoryFailed { request, message } => {
            if !matches!(
                app.package_inventory,
                PackageInventoryState::Loading { request: pending } if pending == request
            ) {
                return None;
            }
            app.package_inventory = PackageInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Package inventory is unavailable: {message}"));
        }
        Action::SelectPackage { delta } => {
            let visible = app
                .filtered_packages()
                .into_iter()
                .map(|package| package.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.package_selection = None;
                return None;
            }
            let current = app
                .package_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = shifted_index(current, delta, visible.len());
            app.package_selection = Some(visible[next].clone());
            app.package_dependency_selection = 0;
        }
        Action::BeginPackageSearch => app.package_searching = true,
        Action::AppendPackageQuery(character) => {
            if !character.is_control() && app.package_query.len() < 256 {
                app.package_query.push(character);
                set_package_selection_to_current_or_first(app, app.package_selection.clone());
            }
        }
        Action::BackspacePackageQuery => {
            app.package_query.pop();
            set_package_selection_to_current_or_first(app, app.package_selection.clone());
        }
        Action::ClearPackageQuery => {
            app.package_query.clear();
            set_package_selection_to_current_or_first(app, app.package_selection.clone());
        }
        Action::FinishPackageSearch => app.package_searching = false,
        Action::BeginSelectedPackageDetail => {
            let Some(identity) = app
                .selected_package()
                .map(|package| package.identity.clone())
            else {
                app.notification = Some("No current package is selected for inspection.".into());
                return None;
            };
            if package_operation_is_loading(app) {
                app.notification = Some("A package-data operation is already running.".into());
                return None;
            }
            return Some(begin_package_detail(app, identity));
        }
        Action::PackageDetailLoaded { request, detail } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            let mut limitations = Vec::new();
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            let state = if !limitations.is_empty() {
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                }
            } else if package_detail_is_empty(&detail) {
                PackageDetailState::AvailableEmpty { request }
            } else {
                PackageDetailState::Available { request, detail }
            };
            app.package_details
                .insert(state.request().unwrap().identity.clone(), state);
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailPartial {
            request,
            detail,
            mut limitations,
        } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            let (detail, report) = normalize_package_detail(&request.identity, detail);
            let Some(detail) = detail else {
                app.package_details.insert(
                    request.identity.clone(),
                    PackageDetailState::Failed {
                        request,
                        message: "backend returned detail for a different or invalid package"
                            .into(),
                    },
                );
                return None;
            };
            append_package_normalization_limitations(&mut limitations, &report);
            let limitations = normalize_package_limitations(limitations);
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Partial {
                    request,
                    detail,
                    limitations,
                },
            );
            app.package_dependency_selection = 0;
        }
        Action::PackageDetailFailed { request, message } => {
            if !app.package_details.get(&request.identity).is_some_and(
                |state| matches!(state, PackageDetailState::Loading { request: pending } if pending == &request),
            ) {
                return None;
            }
            app.package_details.insert(
                request.identity.clone(),
                PackageDetailState::Failed {
                    request,
                    message: message.clone(),
                },
            );
            app.notification = Some(format!("Package detail is unavailable: {message}"));
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
