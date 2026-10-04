use super::*;

fn begin(project: bool) -> (App, u64) {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Hardware;
    app.hardware.selection = 3;
    app.hardware.projects.selection = 4;
    viewer::open(
        &mut app,
        HardwareDocument {
            path: "/data/board/scripts/startup".into(),
            category: HardwareCategory::Other,
            kind: HardwareDocumentKind::Text,
        },
        project.then(|| "/data/board".into()),
    );
    let generation = app.hardware.viewer.as_ref().unwrap().generation;
    (app, generation)
}

fn loaded(app: &mut App, generation: u64, content: &str) {
    update(
        app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation,
            page_count: 1,
            preview: HardwarePreview::Source(content.into()),
            searchable_text: content.lines().map(str::to_owned).collect(),
        }),
    );
}

#[test]
fn hardware_text_opens_editable_source_and_preserves_revision_and_selection() {
    let source = "#!/bin/sh\necho original\n\n";
    for project in [false, true] {
        let (mut app, generation) = begin(project);
        loaded(&mut app, generation, source);
        let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
            panic!("no source editor");
        };
        assert_eq!(
            editor.context,
            if project {
                SourceEditorContext::HardwareProject
            } else {
                SourceEditorContext::HardwareLibrary
            }
        );
        assert_eq!(editor.language, SourceLanguage::Shell);
        assert_eq!(editor.document.text, source);
        assert!(!editor.is_dirty());
        assert_eq!(
            editor.document.base_revision(),
            TextAreaRevision::of(source)
        );
        assert!(app.hardware.viewer.is_none());
        assert!(update(&mut app, Action::BeginRecipeEditorBuild).is_none());
        update(&mut app, Action::ToggleRecipeEditorEditing);
        update(&mut app, Action::AppendRecipeEditor('#'));
        assert!(
            matches!(update(&mut app, Action::SaveRecipeEditor), Some(Effect::SaveRecipeEditorFile { expected, .. }) if expected == TextAreaRevision::of(source))
        );
        update(&mut app, Action::CloseRecipeEditor);
        assert!(app.active_dialog().is_some(), "dirty editor must not close");
        update(&mut app, Action::RecipeEditorSaved);
        update(&mut app, Action::CloseRecipeEditor);
        assert!(app.active_dialog().is_none());
        assert_eq!(app.screen, Screen::Hardware);
        assert_eq!(app.hardware.selection, 3);
        assert_eq!(app.hardware.projects.selection, 4);
    }
}

#[test]
fn hardware_text_stale_inactive_modal_and_closed_results_do_not_take_over() {
    let (mut app, generation) = begin(false);
    loaded(&mut app, generation + 1, "stale");
    assert!(app.active_dialog().is_none());
    assert!(app.hardware.viewer.as_ref().unwrap().loading);
    app.screen = Screen::Dashboard;
    loaded(&mut app, generation, "inactive");
    assert!(app.active_dialog().is_none());
    app.screen = Screen::Hardware;
    app.dialogs.push_back(Dialog::BuildOptions);
    loaded(&mut app, generation, "modal");
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    app.dialogs.clear();
    update(&mut app, Action::Hardware(HardwareAction::CloseViewer));
    loaded(&mut app, generation, "closed");
    assert!(app.active_dialog().is_none());
}

#[test]
fn hardware_text_rejects_binary_controls_and_oversized_typed_content() {
    for source in [
        "nul\0payload".into(),
        "escape\x1b[31m".into(),
        "x".repeat(TEXTAREA_MAX_BYTES + 1),
    ] {
        let (mut app, generation) = begin(false);
        loaded(&mut app, generation, &source);
        assert!(app.active_dialog().is_none());
        assert!(app.hardware.viewer.as_ref().unwrap().error.is_some());
    }
}

#[test]
fn hardware_text_external_reload_is_bounded_and_never_offers_recipe_builds() {
    let (mut app, generation) = begin(false);
    loaded(&mut app, generation, "original\n");
    for source in ["binary\0".into(), "x".repeat(TEXTAREA_MAX_BYTES + 1)] {
        update(&mut app, Action::LoadRecipeEditorExternalContent(source));
        let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
            panic!("editor closed unexpectedly");
        };
        assert_eq!(editor.document.text, "original\n");
    }
    update(
        &mut app,
        Action::LoadRecipeEditorExternalContent("changed\n".into()),
    );
    assert!(!app.notification.as_ref().unwrap().contains("Ctrl+B"));
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("editor closed unexpectedly");
    };
    assert_eq!(editor.document.text, "changed\n");
}

#[test]
fn hardware_text_language_detects_extensions_names_and_shebangs() {
    for (name, content, language) in [
        ("driver.c", "int main() {}", SourceLanguage::C),
        ("Makefile", "all:\n\techo done", SourceLanguage::Make),
        (
            "run",
            "#!/usr/bin/env -S python3 -u\nprint(True)",
            SourceLanguage::Python,
        ),
        (
            "entry.unknown",
            "#!/bin/bash\necho yes",
            SourceLanguage::Shell,
        ),
        ("README", "plain text", SourceLanguage::PlainText),
    ] {
        assert_eq!(
            SourceLanguage::from_source(Path::new(name), content),
            language
        );
        assert!(
            HardwareDocument {
                path: PathBuf::from("/data").join(name),
                category: HardwareCategory::Other,
                kind: HardwareDocumentKind::Text
            }
            .validate()
            .is_ok()
        );
    }
}
