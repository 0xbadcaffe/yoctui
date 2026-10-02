use super::*;

#[test]
fn editor_save_build_navigation_and_search_hints_survive_every_supported_layout_and_mode() {
    for focus in [
        yoctui_model::RecipeEditorFocus::Files,
        yoctui_model::RecipeEditorFocus::Document,
    ] {
        for mode in [
            yoctui_model::TextAreaMode::Normal,
            yoctui_model::TextAreaMode::Insert,
            yoctui_model::TextAreaMode::Visual,
        ] {
            let mut app = App::new(32, 8192);
            let mut document = yoctui_model::TextAreaState::new("int main() {}".into());
            document.set_mode(mode);
            app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
                recipe: "busybox".into(),
                root: "/workspace/busybox".into(),
                files: vec!["main.c".into()],
                file_inventory_truncated: false,
                selection: 0,
                focus,
                language: SourceLanguage::C,
                document,
                searching: false,
                pending_search_position: None,
            }));
            app.focus = FocusTarget::Dialog;
            for (width, height) in [(80, 24), (100, 30), (160, 50), (200, 60)] {
                let output = rendered_text_at(&app, width, height, literal_now());
                for anchor in [
                    "Ctrl+S save",
                    "Ctrl+B build recipe",
                    "Ctrl+F file",
                    "Alt+f workspace",
                    "Alt+g GitUI",
                    "int main() {}",
                ] {
                    assert!(
                        output.contains(anchor),
                        "{focus:?}/{mode:?}/{width}x{height} lost {anchor}: {output}"
                    );
                }
                let navigation = if focus == yoctui_model::RecipeEditorFocus::Files {
                    "Enter edit"
                } else {
                    "Tab files"
                };
                assert!(output.contains(navigation), "{output}");
                let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
                    panic!("render changed dialog")
                };
                assert_eq!(editor.focus, focus);
                assert_eq!(editor.document.mode(), mode);
                assert_eq!(editor.document.text, "int main() {}");
                assert!(!editor.document.is_modified());
                assert_eq!(app.focus, FocusTarget::Dialog);
            }
        }
    }
}

#[test]
fn editor_application_menu_keeps_save_build_and_files_rail_visible() {
    let app = concept_editor_menu_app();
    let output = rendered_text_at(&app, 160, 50, literal_now());
    for anchor in [
        "Yoctui Application Menu",
        "Ctrl+S save",
        "Ctrl+B build recipe",
        "Tab files",
        "Alt+f workspace",
        "Alt+g GitUI",
    ] {
        assert!(output.contains(anchor), "{output}");
    }
}
