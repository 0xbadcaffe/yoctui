use super::*;

pub(super) fn open(
    app: &mut App,
    document: HardwareDocument,
    root: Option<PathBuf>,
) -> Option<Effect> {
    if let Err(error) = document.validate() {
        app.notification = Some(error);
        return None;
    }
    let state = &mut app.hardware;
    let generation = next_generation(&mut state.viewer_generation);
    let viewer = HardwareViewerState {
        document,
        generation,
        page: 1,
        page_count: 1,
        zoom_percent: 100,
        fit_width: false,
        presentation: HardwarePresentation::Page,
        pan_x: 0,
        pan_y: 0,
        loading: true,
        preview: None,
        searchable_text: Vec::new(),
        query: String::new(),
        searching: false,
        matches: Vec::new(),
        match_selection: 0,
        error: None,
    };
    let request = viewer.request();
    state.viewer = Some(viewer);
    state.project_view_root = root;
    app.focus = FocusTarget::Workspace;
    if app.zoomed_pane.is_some() {
        app.zoomed_pane = Some(FocusTarget::Workspace);
    }
    Some(load_effect(state, request))
}

pub(super) fn load_effect(state: &HardwareState, request: HardwareLoadRequest) -> Effect {
    Effect::Hardware(match &state.project_view_root {
        Some(root) => HardwareEffect::LoadProject {
            root: root.clone(),
            request,
        },
        None => HardwareEffect::Load(request),
    })
}

pub(super) fn edit_loaded_source(app: &mut App) {
    if app.screen != Screen::Hardware
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.hardware.browser.is_some()
        || app.hardware.projects.form.is_some()
        || app.hardware.projects.import_browser.is_some()
    {
        return;
    }
    let Some(viewer) = app.hardware.viewer.as_ref() else {
        return;
    };
    let Some(HardwarePreview::Source(content)) = viewer.preview.as_ref() else {
        return;
    };
    let path = &viewer.document.path;
    let Some(parent) = path.parent() else {
        return;
    };
    let (root, context) = app.hardware.project_view_root.as_ref().map_or_else(
        || (parent.to_path_buf(), SourceEditorContext::HardwareLibrary),
        |root| (root.clone(), SourceEditorContext::HardwareProject),
    );
    let Ok(relative) = path.strip_prefix(&root) else {
        return;
    };
    let mut editor = RecipeEditor {
        recipe: format!("Hardware: {}", viewer.document.name()),
        root: root.clone(),
        files: vec![relative.to_path_buf()],
        file_inventory_truncated: false,
        context,
        selection: 0,
        focus: RecipeEditorFocus::Document,
        language: SourceLanguage::from_source(path, content),
        document: TextAreaState::new(content.clone()),
        searching: false,
        pending_search_position: None,
    };
    editor.refresh_language_and_validation();
    editor.document.select_position(0, 0, false);
    app.hardware.viewer = None;
    app.hardware.project_view_root = None;
    open_dialog(app, Dialog::RecipeEditor(editor));
    synchronize_focus(app);
}
