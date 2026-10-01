//! Reviewed environment loading; archives never supply live execution authority.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedEnvironmentMode {
    Start,
    Attach { instance: [u8; 16] },
    Restart { instance: [u8; 16] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedEnvironmentPlan {
    pub profile: BuildEnvironmentProfile,
    pub target: String,
    pub machine: Option<String>,
    pub mode: SavedEnvironmentMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavedEnvironmentState {
    pub generation: u64,
    pub preparing: bool,
    pub loading: bool,
    pub plan: Option<SavedEnvironmentPlan>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedEnvironmentAction {
    Begin,
    Prepared {
        generation: u64,
        result: Result<SavedEnvironmentPlan, String>,
    },
    Confirm,
    Cancel,
    Finished {
        generation: u64,
        result: Result<BuildEnvironmentProfile, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedEnvironmentRequest {
    Prepare {
        generation: u64,
        record: SavedBuild,
    },
    Load {
        generation: u64,
        plan: SavedEnvironmentPlan,
    },
}

pub(crate) fn reduce_saved_environment(
    app: &mut App,
    action: SavedEnvironmentAction,
) -> Option<Effect> {
    match action {
        SavedEnvironmentAction::Begin => {
            if app.active_dialog().is_some()
                || app.saved_builds.environment.preparing
                || app.saved_builds.environment.loading
            {
                return None;
            }
            let record = app
                .saved_builds
                .records
                .get(app.saved_builds.selection)?
                .clone();
            if app
                .notification
                .as_deref()
                .is_some_and(|message| message.starts_with("Load environment failed:"))
            {
                app.notification = None;
            }
            let state = &mut app.saved_builds.environment;
            state.generation = state.generation.checked_add(1)?;
            state.preparing = true;
            state.plan = None;
            state.error = None;
            Some(Effect::SavedEnvironment(SavedEnvironmentRequest::Prepare {
                generation: state.generation,
                record,
            }))
        }
        SavedEnvironmentAction::Prepared { generation, result } => {
            if generation != app.saved_builds.environment.generation
                || !app.saved_builds.environment.preparing
            {
                return None;
            }
            app.saved_builds.environment.preparing = false;
            match result {
                Ok(plan) if app.active_dialog().is_none() && app.screen == Screen::BuildHistory => {
                    app.saved_builds.environment.plan = Some(plan.clone());
                    open_dialog(app, Dialog::SavedEnvironmentReview(plan));
                    synchronize_focus(app);
                }
                Ok(_) => saved_environment_error(
                    app,
                    "Environment review interrupted by navigation or another dialog; press o to retry.".into(),
                ),
                Err(error) => saved_environment_error(app, error),
            }
            None
        }
        SavedEnvironmentAction::Confirm => {
            let Dialog::SavedEnvironmentReview(plan) = app.active_dialog()?.clone() else {
                return None;
            };
            if app.saved_builds.environment.plan.as_ref() != Some(&plan) {
                return None;
            }
            close_dialog(app);
            synchronize_focus(app);
            app.saved_builds.environment.loading = true;
            Some(Effect::SavedEnvironment(SavedEnvironmentRequest::Load {
                generation: app.saved_builds.environment.generation,
                plan,
            }))
        }
        SavedEnvironmentAction::Cancel => {
            if !matches!(app.active_dialog(), Some(Dialog::SavedEnvironmentReview(_))) {
                return None;
            }
            close_dialog(app);
            synchronize_focus(app);
            app.saved_builds.environment.plan = None;
            None
        }
        SavedEnvironmentAction::Finished { generation, result } => {
            if generation != app.saved_builds.environment.generation
                || !app.saved_builds.environment.loading
            {
                return None;
            }
            app.saved_builds.environment.loading = false;
            app.saved_builds.environment.plan = None;
            match result {
                Ok(profile) => {
                    app.build_environment = BuildEnvironmentState::Connected(profile.clone());
                    app.saved_builds.browsing = false;
                    app.saved_builds.view = None;
                    if app.active_dialog().is_none() {
                        let _ = update(app, Action::Open(Screen::Dashboard));
                    }
                    app.notification = Some(format!(
                        "Loaded environment {}. Select a target to start a new build; no saved job was replayed.",
                        profile.build_dir.display()
                    ));
                }
                Err(error) => saved_environment_error(app, error),
            }
            None
        }
    }
}

fn saved_environment_error(app: &mut App, error: String) {
    app.notification = Some(format!("Load environment failed: {error}"));
    app.saved_builds.environment.error = Some(error);
}

/// Forget path-bound inspection results when a fresh daemon selects another
/// environment. Client preferences, saved archives, dialogs and generations stay.
pub fn clear_saved_environment_views(app: &mut App) {
    let fresh = App::new(app.logs.max_entries, app.logs.max_bytes);
    app.dependencies = None;
    app.dependency_graph = fresh.dependency_graph;
    app.dependency_graph_selection = None;
    app.dependency_graph_anchor = None;
    app.dependency_graph_collapsed.clear();
    app.signature_dump = fresh.signature_dump;
    app.signature_comparison = fresh.signature_comparison;
    app.signature_selection = None;
    app.signature_recipe = None;
    app.package_inventory = fresh.package_inventory;
    app.package_details.clear();
    app.package_selection = None;
    app.package_navigation.clear();
    app.image_artifacts = fresh.image_artifacts;
    app.image_artifact_selection = None;
    app.rootfs_composition = fresh.rootfs_composition;
    app.rootfs_group_selection = None;
    app.rootfs_package_selection = None;
    app.rootfs_entry_selection = None;
    app.overview_image_size_history.clear();
    app.sdk_artifacts = fresh.sdk_artifacts;
    app.sdk_artifact_selection = None;
    app.sdk_tool_capability = fresh.sdk_tool_capability;
    app.test_capability = fresh.test_capability;
    app.result_tool_capability = fresh.result_tool_capability;
    app.test_results = fresh.test_results;
    app.test_result_selection = None;
    app.test_case_selection = None;
    app.test_comparison = fresh.test_comparison;
    app.test_junit_export = fresh.test_junit_export;
    app.security = fresh.security;
    app.qa = fresh.qa;
    app.maintenance = fresh.maintenance;
    app.qemu_capability = fresh.qemu_capability;
    app.pending_qemu_launch = false;
    app.wic_capability = fresh.wic_capability;
    app.pending_wic_create = false;
    app.wic_outputs = fresh.wic_outputs;
    app.wic_devices = fresh.wic_devices;
    app.wic_output_selection = None;
    app.wic_device_selection = None;
    app.kernel = fresh.kernel;
    app.firmware = fresh.firmware;
    app.layer_relationships = None;
    app.recipe_sources.clear();
    app.recipe_metadata.clear();
    app.recipe_metadata_loading.clear();
    app.recipe_metadata_errors.clear();
    app.devtool_statuses.clear();
    app.devtool_status_loading.clear();
    app.variable_details.clear();
    app.variable_detail_loading.clear();
    app.variable_detail_errors.clear();
    app.layer_browser = None;
    app.project_profile = fresh.project_profile;
}

#[cfg(test)]
#[path = "tests/saved_environment.rs"]
mod tests;
