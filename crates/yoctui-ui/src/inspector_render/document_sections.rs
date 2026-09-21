pub(crate) fn system_status_text(app: &App, width: u16) -> String {
    system_status_projection(app, width)
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn system_status_document(app: &App, width: u16) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    Text::from(
        system_status_projection(app, width)
            .into_iter()
            .map(|line| Line::styled(line.text, status_tone_style(&palette, line.tone)))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn push_inspector_section(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    title: &'static str,
    body: &str,
) {
    if body.trim().is_empty() {
        return;
    }
    if !lines.is_empty() {
        lines.push(Line::default());
    }
    let palette = ThemePalette::for_app(app);
    lines.push(Line::styled(
        format!("▾ {title}"),
        palette.role(palette.heading, Modifier::BOLD),
    ));
    lines.extend(body.lines().map(|line| Line::from(line.to_owned())));
}

pub(crate) struct InspectorDocumentSections<'a> {
    pub(crate) primary: &'a str,
    pub(crate) secondary: Option<&'a str>,
    pub(crate) related_paths: &'a [String],
    pub(crate) recent_output: Option<&'a str>,
    pub(crate) actions: Option<&'a [ActionListItem]>,
    pub(crate) status: Option<&'a str>,
}

pub(crate) fn inspector_document(
    app: &App,
    sections: InspectorDocumentSections<'_>,
    width: u16,
) -> Text<'static> {
    let mut lines = Vec::new();
    push_inspector_section(&mut lines, app, "PRIMARY FACTS", sections.primary);
    if let Some(secondary) = sections.secondary {
        push_inspector_section(&mut lines, app, "SECONDARY FACTS", secondary);
    }
    if !sections.related_paths.is_empty() {
        push_inspector_section(
            &mut lines,
            app,
            "RELATED PATHS",
            &sections.related_paths.join("\n"),
        );
    }
    if let Some(output) = sections.recent_output {
        push_inspector_section(&mut lines, app, "RECENT OUTPUT", output);
    }
    if let Some(actions) = sections.actions.filter(|actions| !actions.is_empty()) {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        let palette = ThemePalette::for_app(app);
        lines.push(Line::styled(
            "▾ CONTEXTUAL ACTIONS",
            palette.role(palette.heading, Modifier::BOLD),
        ));
        lines.extend(action_list(actions, width, inspector_action_styles(app)).lines);
    }
    if let Some(status) = sections.status {
        push_inspector_section(&mut lines, app, "SYSTEM / COMPATIBILITY", status);
    }
    Text::from(lines)
}

pub(crate) fn inspector_related_paths(app: &App) -> Vec<String> {
    let path = match app.screen {
        Screen::Recipes => app
            .workspace
            .recipes
            .get(app.recipe_selection)
            .and_then(|recipe| recipe.file.clone()),
        Screen::Layers => app
            .layer_browser
            .as_ref()
            .and_then(|browser| {
                browser.selected_entry().map(|entry| {
                    if entry.path.is_absolute() {
                        entry.path.clone()
                    } else {
                        browser.root.join(&entry.path)
                    }
                })
            })
            .or_else(|| {
                app.workspace
                    .layers
                    .get(app.layer_selection)
                    .map(|layer| layer.path.clone())
            }),
        Screen::Logs if app.log_workspace_view == LogWorkspaceView::BitBake => {
            app.logs.selected().and_then(|entry| entry.path.clone())
        }
        Screen::Errors => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .and_then(|entry| entry.path.clone()),
        Screen::Images => app
            .selected_image_artifact()
            .map(|artifact| artifact.identity.path.clone()),
        Screen::Kernel => app.kernel.selected_file().map(|file| file.path.clone()),
        Screen::Firmware => app.firmware.selected_file().map(|file| file.path.clone()),
        Screen::Sdk => app
            .selected_sdk_artifact()
            .map(|artifact| artifact.identity.path.clone()),
        Screen::BuildHistory if app.is_offline() || app.saved_builds.browsing => None,
        Screen::BuildHistory => app
            .job_history_rows()
            .get(app.build_history_selection)
            .and_then(|row| match row {
                JobHistoryRowRef::Daemon(_) => None,
                JobHistoryRowRef::Background(job) => job.context.path.clone(),
                JobHistoryRowRef::Build(_) => None,
            }),
        _ => None,
    };
    path.into_iter()
        .map(|path| path.display().to_string())
        .collect()
}
