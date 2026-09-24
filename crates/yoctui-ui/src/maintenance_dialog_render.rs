//! Maintenance dialog render.
use super::*;

pub(crate) fn maintenance_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &MaintenanceDialog,
    area: Rect,
) {
    let palette = ThemePalette::for_app(app);
    let (title, body, style) = match dialog {
        MaintenanceDialog::ReadinessToml { .. } => {
            unreachable!("sstate readiness uses the shared editor")
        }
        MaintenanceDialog::CleanupToml { .. } => {
            unreachable!("sstate cleanup uses the shared editor")
        }
        MaintenanceDialog::PrServiceToml { .. } => {
            unreachable!("PR service forms use the shared editor")
        }
        MaintenanceDialog::LockedCacheToml { .. } => {
            unreachable!("locked-cache form uses the shared editor")
        }
        MaintenanceDialog::BuildHistoryToml { .. } => unreachable!("build-history form uses the shared editor"),
        MaintenanceDialog::GitArchiveToml { .. } => {
            unreachable!("Git archive form uses the shared editor")
        }
        MaintenanceDialog::ReadinessForm(draft) => (
            "Sstate readiness check",
            format!(
                "{} Targets: {}\n{} Mode: {:?}\n{} Output: {}\n{} Log: {}\n{} Timeout seconds: {}\n\n{}\n\nTab/Shift+Tab field | Space/←/→ mode | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceReadinessField::Targets { "▶" } else { " " },
                if draft.targets.is_empty() { "<required>" } else { &draft.targets },
                if draft.field == yoctui_model::MaintenanceReadinessField::Mode { "▶" } else { " " },
                draft.mode,
                if draft.field == yoctui_model::MaintenanceReadinessField::Output { "▶" } else { " " },
                if draft.output.is_empty() { "<none>" } else { &draft.output },
                if draft.field == yoctui_model::MaintenanceReadinessField::Log { "▶" } else { " " },
                if draft.log.is_empty() { "<none>" } else { &draft.log },
                if draft.field == yoctui_model::MaintenanceReadinessField::Timeout { "▶" } else { " " },
                if draft.timeout.is_empty() { "<required>" } else { &draft.timeout },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: no command runs until the exact adapter preview is confirmed.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else {
                palette.role(palette.informational, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::CleanupForm(draft) => (
            "Protected sstate cleanup preview",
            format!(
                "Cache: {}\nStamps:\n- {}\n\n{} [{}] duplicates\n{} [{}] orphans\n{} [{}] unreferenced by stamps\n{} Jobs: {}\n\n{}\n\nTab/Shift+Tab field | Space toggle | Enter discover candidates | Esc cancel",
                draft.cache_dir.display(),
                if draft.stamps_dirs.is_empty() { "none".into() } else { draft.stamps_dirs.iter().take(3).map(|path| path.display().to_string()).collect::<Vec<_>>().join("\n- ") },
                if draft.field == yoctui_model::MaintenanceCleanupField::Duplicates { "▶" } else { " " },
                if draft.duplicates { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::Orphans { "▶" } else { " " },
                if draft.orphans { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::UnreferencedByStamps { "▶" } else { " " },
                if draft.unreferenced_by_stamps { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceCleanupField::Jobs { "▶" } else { " " },
                if draft.jobs.is_empty() { "<required>" } else { &draft.jobs },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: candidate discovery is read-only. Deletion still requires the exact phrase and a second confirmation.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else {
                palette.role(palette.warning, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::PrServiceForm(draft) => (
            match draft.operation {
                yoctui_model::PrServiceOperation::Export => "PR service export",
                yoctui_model::PrServiceOperation::Import => "PR service import",
            },
            format!(
                "Operation: {:?}\nFile: {}\nBuild directory: {}\nConfigured endpoint: {}\n\nThe native helper may stop a memory-resident BitBake server and invalidate BitBake cache records.{}\n\n{}\n\nType canonical .conf/.inc path | Enter preview | Esc cancel",
                draft.operation,
                if draft.file.is_empty() { "<required>" } else { &draft.file },
                draft.build_dir.display(),
                draft.endpoint,
                if draft.operation == yoctui_model::PrServiceOperation::Import {
                    "\nImport changes PR service data."
                } else {
                    "\nExport may replace the exact destination."
                },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: no helper runs until the exact adapter preview is confirmed.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            if draft.validation.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else if draft.operation == yoctui_model::PrServiceOperation::Import {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.informational, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::LockedCacheForm(draft) => (
            "Locked-signature cache",
            format!(
                "{} Locked signatures: {}\n{} Input cache: {}\n{} Output cache: {}\n  Native LSB (read-only): {}\n{} Filter: {}\n\nMatching files beneath the exact output cache may be replaced. The adapter preview and a separate destructive confirmation are still required.\n\n{}\n\nTab/Shift+Tab field | Type/Backspace edit | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceLockedCacheField::LockedSignatures { "▶" } else { " " },
                if draft.locked_signatures.is_empty() { "<required>" } else { &draft.locked_signatures },
                if draft.field == yoctui_model::MaintenanceLockedCacheField::InputCache { "▶" } else { " " },
                if draft.input_cache.is_empty() { "<required>" } else { &draft.input_cache },
                if draft.field == yoctui_model::MaintenanceLockedCacheField::OutputCache { "▶" } else { " " },
                if draft.output_cache.is_empty() { "<required>" } else { &draft.output_cache },
                draft.native_lsb,
                if draft.field == yoctui_model::MaintenanceLockedCacheField::Filter { "▶" } else { " " },
                if draft.filter.is_empty() { "<none>" } else { &draft.filter },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: no generator runs until the exact adapter preview is confirmed.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else {
                    palette.warning
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::BuildHistoryForm(draft) => (
            "Build-history comparison",
            format!(
                "Repository (read-only): {}\n{} From revision: {}\n{} To revision: {}\n{} [{}] report version\n{} [{}] report all\n{} [{}] signatures\n{} [{}] signature diff\n{} Exclude paths: {}\n{} [{}] no colour\n\nComparison uses buildhistory-diff only; build-compare is a separate unsupported capability. Output is bounded session evidence.\n\n{}\n\nTab/Shift+Tab field | Space/←/→ toggle | Enter preview | Esc cancel",
                draft.repository.display(),
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::FromRevision { "▶" } else { " " },
                if draft.from_revision.is_empty() { "<none>" } else { &draft.from_revision },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ToRevision { "▶" } else { " " },
                if draft.to_revision.is_empty() { "<none>" } else { &draft.to_revision },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ReportVersion { "▶" } else { " " },
                if draft.report_version { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ReportAll { "▶" } else { " " },
                if draft.report_all { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::Signatures { "▶" } else { " " },
                if draft.signatures { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::SignatureDiff { "▶" } else { " " },
                if draft.signature_diff { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::ExcludePaths { "▶" } else { " " },
                if draft.exclude_paths.is_empty() { "<none>" } else { &draft.exclude_paths },
                if draft.field == yoctui_model::MaintenanceBuildHistoryField::NoColour { "▶" } else { " " },
                if draft.no_colour { "x" } else { " " },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: no comparison runs until the exact adapter preview is confirmed.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else {
                    palette.informational
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::GitArchiveForm(draft) => (
            "Git release archive",
            format!(
                "{} Data directory: {}\n{} Git directory: {}\n{} [{}] create  {} [{}] bare  {} [{}] create tag\n{} Branch: {}\n{} Tag: {}\n{} Commit subject: {}\n{} Commit body: {}\n{} Tag subject: {}\n{} Tag body: {}\n{} Exclusions: {}\n{} Notes: {}\n{} Push remote: {}\n\nLocal archive creation runs first. A configured push is deferred and requires a second network confirmation after local success. Repository creation, tag replacement, and tracked-output overwrite risks remain visible in the adapter preview.\n\n{}\n\nTab/Shift+Tab field | Space/←/→ toggle | Enter preview | Esc cancel",
                if draft.field == yoctui_model::MaintenanceGitArchiveField::DataDir { "▶" } else { " " },
                if draft.data_dir.is_empty() { "<required>" } else { &draft.data_dir },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::GitDir { "▶" } else { " " },
                if draft.git_dir.is_empty() { "<required>" } else { &draft.git_dir },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Create { "▶" } else { " " },
                if draft.create { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Bare { "▶" } else { " " },
                if draft.bare { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CreateTag { "▶" } else { " " },
                if draft.create_tag { "x" } else { " " },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::BranchName { "▶" } else { " " },
                draft.branch_name,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagName { "▶" } else { " " },
                if draft.tag_name.is_empty() { "<none>" } else { &draft.tag_name },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CommitSubject { "▶" } else { " " },
                draft.commit_subject,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::CommitBody { "▶" } else { " " },
                if draft.commit_body.is_empty() { "<none>" } else { &draft.commit_body },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagSubject { "▶" } else { " " },
                draft.tag_subject,
                if draft.field == yoctui_model::MaintenanceGitArchiveField::TagBody { "▶" } else { " " },
                if draft.tag_body.is_empty() { "<none>" } else { &draft.tag_body },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Exclusions { "▶" } else { " " },
                if draft.exclusions.is_empty() { "<none>" } else { &draft.exclusions },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::Notes { "▶" } else { " " },
                if draft.notes.is_empty() { "<none>" } else { &draft.notes },
                if draft.field == yoctui_model::MaintenanceGitArchiveField::PushRemote { "▶" } else { " " },
                if draft.push_remote.is_empty() { "<local only>" } else { &draft.push_remote },
                draft.validation.as_ref().map_or_else(
                    || "✓ Validation: no archive or network operation runs until the exact preview is confirmed.".into(),
                    |error| format!("✕ Validation: {error}"),
                ),
            ),
            palette.role(
                if draft.validation.is_some() {
                    palette.error
                } else if !draft.push_remote.is_empty() {
                    palette.warning
                } else {
                    palette.informational
                },
                Modifier::BOLD,
            ),
        ),
        MaintenanceDialog::Confirm(preview) => (
            if preview.operation.destructive() {
                "Confirm destructive Maintenance operation"
            } else {
                "Confirm Maintenance operation"
            },
            format!(
                "Operation {}\nKind: {}\nDestructive: {}\nNetwork: {}\n\nIndexed native vector:\n{}\n\nLimitations:\n- {}\n\nEnter confirms | Esc cancels",
                preview.id,
                maintenance_operation_label(&preview.operation),
                preview.operation.destructive(),
                preview.operation.network_side_effect(),
                indexed_arguments(&preview.arguments),
                if preview.limitations.is_empty() {
                    "none".into()
                } else {
                    preview.limitations.join("\n- ")
                },
            ),
            if preview.operation.destructive() {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.informational, Modifier::BOLD)
            },
        ),
        MaintenanceDialog::CleanupPhrase { preview, input } => (
            "Confirm protected sstate cleanup",
            format!(
                "Type exactly:\n{}\n\n{input}_\n\nTyping alone cannot delete files.\nEnter continues | Esc cancels",
                preview.operation.cleanup_phrase().unwrap_or_default()
            ),
            palette.role(palette.error, Modifier::BOLD),
        ),
        MaintenanceDialog::ConfirmNetworkPush(preview) => (
            "Confirm network push",
            format!(
                "Operation {} requests a separately confirmed remote push.\n\n{}\n\nEnter confirms | Esc cancels",
                preview.id,
                indexed_arguments(&preview.arguments)
            ),
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
        ),
        MaintenanceDialog::ConfirmCancellation(id) => (
            "Cancel Maintenance operation",
            format!(
                "Cancel exact session {}?\nA cleanup may leave a partially cleaned cache.\n\nEnter confirms | Esc keeps running",
                id.0
            ),
            palette.role(palette.warning, Modifier::BOLD),
        ),
    };
    let width = 78.min(area.width.saturating_sub(2));
    let preferred_height = if matches!(dialog, MaintenanceDialog::GitArchiveForm(_)) {
        22
    } else {
        18
    };
    let popup = dialog_popup_rect(area, width, preferred_height);
    let tone = match dialog {
        MaintenanceDialog::Confirm(preview) if preview.operation.destructive() => {
            DialogTone::Destructive
        }
        MaintenanceDialog::CleanupPhrase { .. } | MaintenanceDialog::ConfirmNetworkPush(_) => {
            DialogTone::Destructive
        }
        MaintenanceDialog::Confirm(_) | MaintenanceDialog::ConfirmCancellation(_) => {
            DialogTone::Confirmation
        }
        _ => DialogTone::Standard,
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(body)
            .style(style)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn maintenance_operation_label(operation: &MaintenanceOperation) -> &'static str {
    match operation {
        MaintenanceOperation::SstateReadiness(_) => "sstate readiness",
        MaintenanceOperation::SstateCleanup(_) => "sstate cleanup",
        MaintenanceOperation::PrService(_) => "PR service",
        MaintenanceOperation::LockedSignatureCache(_) => "locked signature cache",
        MaintenanceOperation::BuildHistoryComparison(_) => "build-history comparison",
        MaintenanceOperation::BuildCompare(_) => "build compare",
        MaintenanceOperation::GitArchive(_) => "Git archive",
    }
}
pub(crate) fn bbmask_assignment(value: &str) -> String {
    format!(
        "BBMASK = \"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}
pub(crate) fn help(frame: &mut Frame, app: &App, area: Rect) {
    let function_keys = FUNCTION_SHORTCUTS
        .chunks(5)
        .map(|shortcuts| {
            shortcuts
                .iter()
                .map(|shortcut| format!("{} {}", shortcut.key_label, shortcut.action_label))
                .collect::<Vec<_>>()
                .join("   ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let catalog_actions = yoctui_model::global_operator_action_definitions()
        .into_iter()
        .filter(|action| action.shortcut != "none")
        .map(|action| {
            format!(
                "[{}] {:>10}  {:<24}  {}",
                action.help_group.label(),
                action.shortcut,
                action.label,
                action.menu_path.join(" > ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!(
        "Operator guide\n1. Configure the build environment, machine, and image from F12 > Workspace/Build.\n2. Start and monitor work from Dashboard or Tasks; inspect failures in Errors and Logs.\n3. Browse Recipes, Images, Kernel, and Firmware. Use F12 > Actions for operations on the current screen.\n4. Use Devtool modify to create a workspace source tree, edit/build it, then update-recipe or finish into a layer.\n5. Use QEMU / Wic to boot a compatible deployed image or create/write Wic media.\n\nAbout Yoctui\nYoctui {}\nBuild SHA: {}\nA terminal workbench for operating, inspecting, building, and debugging Yocto/OpenBMC workspaces.\n\nGlobal shortcuts\n{function_keys}\n\nGlobal action shortcuts\n{catalog_actions}\n\nNavigation: arrows move, Enter opens, Esc returns, F12 opens the application menu, ?/F1 opens this screen, q quits.\nSearch: / opens content search; type a regular expression and Enter opens the selected matching file.\nTerminal writers and editors retain literal keys; terminal search is Ctrl+B /.\nDestructive and publishing operations show an exact confirmation before execution.",
        env!("CARGO_PKG_VERSION"),
        env!("YOCTUI_BUILD_SHA")
    );
    frame.render_widget(
        Paragraph::new(text)
            .style(ThemePalette::for_app(app).base())
            .block(Block::default().title("Help").borders(Borders::ALL)),
        area,
    )
}
