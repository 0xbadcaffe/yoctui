// Shared fixtures and regression modules.

use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::{fs, path::PathBuf};
use yoctui_model::{Action, BuildRequest, update};
mod compatibility_ui_inspector_renders_identity_all_states_and_exact_evidence;
mod concept_screen_contracts_render_through_production_renderer;
mod config_compare_renders_typed_outcomes_and_disabled_reason_responsively;
mod dashboard_reuses_task_resource_meters;
mod dependency_graph_renders_typed_partial_paths_and_responsive_modes;
mod devtool_editor_viewport;
mod devtool_workspace;
mod global_search_selection_moves_within_pages;
mod inspector_shell_names_modes_and_orders_typed_sections;
mod keymap_preferences_render_search_custom_capture_errors_and_narrow_state;
mod log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely;
mod palette_retains_typed_facts_at_every_breakpoint;
mod qa_workflow_renders_both_views_findings_inspector_themes_and_breakpoints;
mod raw_form_closes_on_authority_loss_without_preview_or_execution;
mod rootfs_packages_pair_wide_pie_with_exact_table_and_accessible_fallbacks;
mod semantic_snapshots_cover_required_workspaces_and_dialog_families;
mod telemetry_strip_composes_wide_medium_and_hidden_tiers;
mod test_workflow_dialogs_render_exact_previews_at_responsive_boundaries;
mod transient_status_reserves_shortcuts_and_degrades_responsively;
mod wic_device_write_renders_protected_dialogs_inventory_history_and_footer;
mod yocto_utility;

use super::dependency_render::dependency_tree_label;
use super::footer::{footer_item_width, footer_shortcuts, responsive_footer_shortcuts};
use super::header::{HeaderMode, header_mode, transient_status_tone};
use super::image_render::image_artifacts_workspace;
use super::inspector_render::system_status_projection;
use super::inspector_workspace::tasks_inspector;
use super::job_history::JobHistoryColumn;
use super::log_render::log_search_spans;
use super::package_render::{layer_browser_left_width, layer_tree_widget_projection};
use super::palette_render::command_palette_rect;
use super::security_render::security_session_status_label;
use super::task_render::{
    TaskTableColumn, task_state_style, task_table_columns, task_table_row_style,
};
use super::telemetry_gauges::{render_disk_io, render_network_io};
mod golden_support;
use golden_support::*;

mod concept_fixtures;
use concept_fixtures::*;

mod render_support;
use render_support::*;

mod workflow_fixtures;
use workflow_fixtures::*;

mod maintenance_fixtures;
use maintenance_fixtures::*;

mod raw_fixtures;
use raw_fixtures::*;

mod concept_layout_geometry;

use super::telemetry_strip::{TelemetryStripMode, telemetry_strip_mode};
