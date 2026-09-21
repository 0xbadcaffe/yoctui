use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::OpenPackageDependency { identity, reverse } => {
            let available = app
                .selected_package_detail()
                .and_then(PackageDetailState::detail)
                .and_then(|detail| {
                    if reverse {
                        detail.reverse_dependencies.available()
                    } else {
                        detail.runtime_dependencies.available()
                    }
                })
                .is_some_and(|dependencies| dependencies.contains(&identity));
            if !available {
                app.notification = Some(
                    "The requested package dependency is not in the current typed detail.".into(),
                );
                return None;
            }
            if app
                .package_inventory
                .packages()
                .is_some_and(|packages| packages.iter().any(|package| package.identity == identity))
            {
                return select_package_identity(app, identity, false);
            } else {
                app.notification =
                    Some("The dependency is not present in the current package inventory.".into());
            }
        }
        Action::TogglePackageDependencyKind => {
            app.package_dependency_reverse = !app.package_dependency_reverse;
            app.package_dependency_selection = 0;
        }
        Action::SelectPackageDependency { delta } => {
            let count = app
                .selected_package_dependencies()
                .map_or(0, <[PackageIdentity]>::len);
            app.package_dependency_selection = if delta.is_negative() {
                app.package_dependency_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.package_dependency_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::OpenSelectedPackageDependency => {
            let Some(identity) = app.selected_package_dependency().cloned() else {
                app.notification = Some(format!(
                    "No {} dependency is selected.",
                    if app.package_dependency_reverse {
                        "reverse"
                    } else {
                        "runtime"
                    }
                ));
                return None;
            };
            return select_package_identity(app, identity, true);
        }
        Action::BackPackageNavigation => {
            let Some(identity) = app.package_navigation.pop() else {
                app.notification = Some("Package navigation history is empty.".into());
                return None;
            };
            app.package_selection = Some(identity);
            app.package_dependency_selection = 0;
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
