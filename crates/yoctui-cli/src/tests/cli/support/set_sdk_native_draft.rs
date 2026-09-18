use super::*;

pub(crate) fn set_sdk_native_draft(app: &mut App, draft: yoctui_model::SdkNativeDraft) {
    let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() else {
        panic!("SDK native TOML editor");
    };
    let mode = match draft.mode {
        yoctui_model::SdkNativeMode::FindSysroot => "find-sysroot",
        yoctui_model::SdkNativeMode::RunNative => "run-native",
    };
    editor.text = format!(
        "mode = \"{mode}\"\nworkspace = \"{}\"\nrecipe = \"{}\"\ntool = \"{}\"\narguments = \"{}\"\n",
        draft.extracted_root,
        draft.recipe,
        draft.tool,
        draft.arguments.join(" ")
    );
    editor.cursor = editor.text.len();
}
