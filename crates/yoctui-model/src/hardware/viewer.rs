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
