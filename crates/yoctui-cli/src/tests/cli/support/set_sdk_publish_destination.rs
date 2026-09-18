use super::*;

pub(crate) fn set_sdk_publish_destination(app: &mut App, destination: &Path) {
    let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() else {
        panic!("SDK publication TOML editor");
    };
    editor.text = format!("destination = \"{}\"\n", destination.display());
    editor.cursor = editor.text.len();
}
