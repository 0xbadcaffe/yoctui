use super::*;

pub(crate) fn reduce_hardware(app: &mut App, action: HardwareAction) -> Option<Effect> {
    if let HardwareAction::Project(action) = action {
        return projects::reduce_hardware_project(app, action);
    }
    if matches!(
        action,
        HardwareAction::OpenSelected | HardwareAction::EditSelected
    ) {
        let edit = matches!(action, HardwareAction::EditSelected);
        let (mut document, root) = if edit && let Some(viewer) = &app.hardware.viewer {
            (
                viewer.document.clone(),
                app.hardware.project_view_root.clone(),
            )
        } else if edit && app.hardware.projects.visible {
            let project = app.hardware.projects.project.as_ref()?;
            let entry = app
                .hardware
                .projects
                .entries
                .get(app.hardware.projects.selection)?;
            if entry.is_directory {
                return None;
            }
            (
                HardwareDocument {
                    path: entry.path.clone(),
                    category: HardwareCategory::Other,
                    kind: HardwareDocumentKind::Text,
                },
                Some(project.root.clone()),
            )
        } else {
            (app.hardware.selected_document()?.clone(), None)
        };
        if edit {
            document.kind = HardwareDocumentKind::Text;
        }
        return viewer::open(app, document, root);
    }
    let state = &mut app.hardware;
    match action {
        HardwareAction::Project(_) => unreachable!("project actions are routed above"),
        HardwareAction::GraphicsCapabilityDetected(capability) => {
            state.graphics_capability = capability;
        }
        HardwareAction::Install(documents) => match validate_hardware_documents(&documents) {
            Ok(()) => {
                state.documents = documents;
                state.missing_paths.retain(|path| {
                    state
                        .documents
                        .iter()
                        .any(|document| &document.path == path)
                });
                state.selection = 0;
            }
            Err(message) => app.notification = Some(message),
        },
        HardwareAction::SelectCategory { delta } => {
            let index = HardwareCategory::ALL
                .iter()
                .position(|category| category == &state.category)
                .unwrap_or(0);
            state.category =
                HardwareCategory::ALL[shifted_index(index, delta, HardwareCategory::ALL.len())];
            state.selection = 0;
        }
        HardwareAction::SelectDocument { delta } => {
            state.selection =
                shifted_index(state.selection, delta, state.visible_documents().len());
        }
        HardwareAction::OpenBrowser { directory } => {
            let generation = state
                .browser
                .as_ref()
                .map_or(1, |browser| browser.generation.wrapping_add(1).max(1));
            state.browser = Some(HardwareBrowserState {
                directory: directory.clone(),
                entries: Vec::new(),
                selection: 0,
                category: state.category,
                generation,
                loading: true,
                error: None,
            });
            return Some(Effect::Hardware(HardwareEffect::Browse {
                generation,
                directory,
            }));
        }
        HardwareAction::BrowserLoaded {
            generation,
            directory,
            mut entries,
        } => {
            if let Some(browser) = &mut state.browser
                && browser.generation == generation
            {
                entries.truncate(MAX_HARDWARE_BROWSER_ENTRIES);
                browser.directory = directory;
                browser.entries = entries;
                browser.selection = 0;
                browser.loading = false;
                browser.error = None;
            }
        }
        HardwareAction::BrowserFailed {
            generation,
            message,
        } => {
            if let Some(browser) = &mut state.browser
                && browser.generation == generation
            {
                browser.loading = false;
                browser.error = Some(message);
            }
        }
        HardwareAction::SelectBrowserEntry { delta } => {
            if let Some(browser) = &mut state.browser {
                browser.selection = shifted_index(browser.selection, delta, browser.entries.len());
            }
        }
        HardwareAction::SelectBrowserCategory { delta } => {
            if let Some(browser) = &mut state.browser {
                let index = HardwareCategory::ALL
                    .iter()
                    .position(|value| value == &browser.category)
                    .unwrap_or(0);
                browser.category =
                    HardwareCategory::ALL[shifted_index(index, delta, HardwareCategory::ALL.len())];
            }
        }
        HardwareAction::EnterBrowserEntry => {
            let browser = state.browser.as_mut()?;
            let entry = browser.entries.get(browser.selection)?.clone();
            if entry.is_directory {
                browser.generation = next_generation(&mut browser.generation);
                browser.loading = true;
                browser.error = None;
                return Some(Effect::Hardware(HardwareEffect::Browse {
                    generation: browser.generation,
                    directory: entry.path,
                }));
            }
        }
        HardwareAction::BrowseParent => {
            let browser = state.browser.as_mut()?;
            let parent = browser.directory.parent()?.to_path_buf();
            browser.generation = next_generation(&mut browser.generation);
            browser.loading = true;
            browser.error = None;
            return Some(Effect::Hardware(HardwareEffect::Browse {
                generation: browser.generation,
                directory: parent,
            }));
        }
        HardwareAction::ConfirmAdd => {
            let browser = state.browser.as_ref()?;
            let entry = browser.entries.get(browser.selection)?;
            let kind = entry.kind?;
            if entry.is_directory
                || state
                    .documents
                    .iter()
                    .any(|document| document.path == entry.path)
            {
                app.notification = Some("That Hardware document is already in the library.".into());
                return None;
            }
            let document = HardwareDocument {
                path: entry.path.clone(),
                category: browser.category,
                kind,
            };
            if let Err(message) = document.validate() {
                app.notification = Some(message);
                return None;
            }
            if state.documents.len() >= MAX_HARDWARE_DOCUMENTS {
                app.notification = Some(format!(
                    "Hardware library is limited to {MAX_HARDWARE_DOCUMENTS} documents."
                ));
                return None;
            }
            state.last_directory = Some(browser.directory.clone());
            state.category = browser.category;
            state.documents.push(document);
            state.documents.sort_by_key(|document| {
                (
                    HardwareCategory::ALL
                        .iter()
                        .position(|value| value == &document.category),
                    document.name().to_lowercase(),
                )
            });
            state.selection = state.visible_documents().len().saturating_sub(1);
            state.browser = None;
            return Some(Effect::Hardware(HardwareEffect::Persist(
                state.documents.clone(),
            )));
        }
        HardwareAction::CancelBrowser => state.browser = None,
        HardwareAction::OpenSelected | HardwareAction::EditSelected => {
            unreachable!("viewer creation routed above")
        }
        HardwareAction::Reload => {
            let viewer = state.viewer.as_mut()?;
            viewer.generation = next_generation(&mut viewer.generation);
            viewer.loading = true;
            viewer.error = None;
            state.viewer_generation = viewer.generation;
            let request = viewer.request();
            return Some(viewer::load_effect(state, request));
        }
        HardwareAction::PreviewLoaded {
            generation,
            page_count,
            preview,
            mut searchable_text,
        } => {
            let viewer = state.viewer.as_mut()?;
            if viewer.generation != generation {
                return None;
            }
            if let HardwarePreview::Source(content) = &preview
                && (viewer.document.kind != HardwareDocumentKind::Text
                    || content.len() > TEXTAREA_MAX_BYTES
                    || !hardware_source_is_text(content))
            {
                viewer.loading = false;
                viewer.error =
                    Some("Hardware source is binary or exceeds the 1 MiB editor limit.".into());
                return None;
            }
            if let HardwarePreview::Raster(raster) = &preview
                && let Err(message) = raster.validate()
            {
                viewer.loading = false;
                viewer.error = Some(message);
                return None;
            }
            let mut bytes = 0usize;
            searchable_text.retain(|line| {
                bytes = bytes.saturating_add(line.len());
                bytes <= MAX_HARDWARE_TEXT_BYTES
            });
            viewer.page_count = page_count.max(1);
            viewer.page = viewer.page.min(viewer.page_count).max(1);
            viewer.preview = Some(preview);
            viewer.searchable_text = searchable_text;
            viewer.loading = false;
            viewer.error = None;
            viewer.rebuild_matches();
            state.missing_paths.remove(&viewer.document.path);
            viewer::edit_loaded_source(app);
        }
        HardwareAction::PreviewFailed {
            generation,
            message,
        } => {
            let viewer = state.viewer.as_mut()?;
            if viewer.generation == generation {
                viewer.loading = false;
                viewer.error = Some(message);
            }
        }
        HardwareAction::ChangePage { delta } => {
            let viewer = state.viewer.as_mut()?;
            let page = shifted_index(viewer.page.saturating_sub(1), delta, viewer.page_count) + 1;
            if page != viewer.page {
                viewer.page = page;
                viewer.generation = next_generation(&mut viewer.generation);
                viewer.loading = true;
                viewer.pan_x = 0;
                viewer.pan_y = 0;
                state.viewer_generation = viewer.generation;
                let request = viewer.request();
                return Some(viewer::load_effect(state, request));
            }
        }
        HardwareAction::FirstPage => {
            return reduce_hardware(app, HardwareAction::ChangePage { delta: isize::MIN });
        }
        HardwareAction::LastPage => {
            return reduce_hardware(app, HardwareAction::ChangePage { delta: isize::MAX });
        }
        HardwareAction::Zoom { delta } => {
            let viewer = state.viewer.as_mut()?;
            viewer.zoom_percent = viewer
                .zoom_percent
                .saturating_add_signed(delta)
                .clamp(25, 400);
        }
        HardwareAction::ResetZoom => {
            let viewer = state.viewer.as_mut()?;
            viewer.zoom_percent = 100;
            viewer.pan_x = 0;
            viewer.pan_y = 0;
        }
        HardwareAction::TogglePresentation => {
            let viewer = state.viewer.as_mut()?;
            if matches!(
                viewer.document.kind,
                HardwareDocumentKind::Pdf
                    | HardwareDocumentKind::Altium
                    | HardwareDocumentKind::Expedition
            ) {
                viewer.presentation = match viewer.presentation {
                    HardwarePresentation::Page if !viewer.searchable_text.is_empty() => {
                        HardwarePresentation::Text
                    }
                    HardwarePresentation::Page => HardwarePresentation::Page,
                    HardwarePresentation::Text => HardwarePresentation::Page,
                };
                viewer.pan_x = 0;
                viewer.pan_y = 0;
            }
        }
        HardwareAction::Pan {
            horizontal,
            vertical,
        } => {
            let viewer = state.viewer.as_mut()?;
            viewer.pan_x = viewer.pan_x.saturating_add_signed(horizontal);
            viewer.pan_y = viewer.pan_y.saturating_add_signed(vertical);
        }
        HardwareAction::BeginSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.searching = true;
            }
        }
        HardwareAction::AppendSearch(character) => {
            if let Some(viewer) = &mut state.viewer
                && viewer.query.chars().count() < MAX_HARDWARE_QUERY_CHARS
            {
                viewer.query.push(character);
                viewer.rebuild_matches();
            }
        }
        HardwareAction::BackspaceSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.query.pop();
                viewer.rebuild_matches();
            }
        }
        HardwareAction::FinishSearch => {
            if let Some(viewer) = &mut state.viewer {
                viewer.searching = false;
            }
        }
        HardwareAction::NextMatch { backwards } => {
            if let Some(viewer) = &mut state.viewer
                && !viewer.matches.is_empty()
            {
                viewer.match_selection = if backwards {
                    viewer
                        .match_selection
                        .checked_sub(1)
                        .unwrap_or(viewer.matches.len() - 1)
                } else {
                    (viewer.match_selection + 1) % viewer.matches.len()
                };
                viewer.pan_y = viewer.matches[viewer.match_selection];
            }
        }
        HardwareAction::CloseViewer => state.viewer = None,
        HardwareAction::BeginRemove => state.removal_pending = state.selected_document().cloned(),
        HardwareAction::ConfirmRemove => {
            let document = state.removal_pending.take()?;
            state
                .documents
                .retain(|candidate| candidate.path != document.path);
            state.missing_paths.remove(&document.path);
            state.selection = state
                .selection
                .min(state.visible_documents().len().saturating_sub(1));
            return Some(Effect::Hardware(HardwareEffect::Persist(
                state.documents.clone(),
            )));
        }
        HardwareAction::CancelRemove => state.removal_pending = None,
    }
    None
}
