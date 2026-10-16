pub(crate) fn raw_mode_shortcuts(app: &App) -> &'static str {
    use yoctui_model::{RawAttachmentState, RawInteractionMode, RawModeView as View};
    let state = &app.raw_mode;
    if state.favorite_rename.is_some() {
        return "Enter save | Esc cancel | Ctrl+U clear | Ctrl+V paste | Backspace delete";
    }
    if state.favorite_confirmation.is_some() {
        return "Enter remove favorite | Esc cancel (no files deleted)";
    }
    if state.recipe_picker.is_some() {
        return "Esc keep value | Type recipe | Up/Down select | PgUp/PgDn | Home/End | Ctrl+U clear | Enter use recipe";
    }
    if state.output.searching || state.search.editing {
        return "Type query | Enter finish | Esc finish | Ctrl+U clear | Backspace delete";
    }
    match state.view {
        View::Browser => "Esc back | Enter review | / filter commands | Alt+h history | Alt+f favorites | f favorite | Ctrl+U clear | Left/Right pane | PgUp/PgDn",
        View::History => "Esc back | Enter retained output | r review NEW command | Up/Down select | PgUp/PgDn | Home/End",
        View::Favorites => "Esc back | Enter review | r rename | [/] reorder | x remove | i inspect | Up/Down select | PgUp/PgDn",
        View::Form => "Tab field | Enter review | Esc normal/back | i edit | e replace | Left/Right cursor/choice",
        View::Preview => "Esc edit | PgUp/PgDn review | Home/End | Enter run",
        View::Execution => {
            let Some(execution) = state.selected_execution() else { return "Esc back | Waiting for daemon acknowledgement"; };
            if execution.request.interaction == RawInteractionMode::InteractivePty {
                return if execution.phase.is_terminal() {
                    "Esc back | o terminal history"
                } else if execution.attachment == RawAttachmentState::Detached {
                    "Esc back | o terminal | c cancel | r reattach"
                } else {
                    "Esc back | o terminal | c cancel | d detach"
                };
            }
            if execution.phase.is_terminal() {
                "Esc back | / find output | 1/2 stream | f follow | PgUp/PgDn | Home/End | Left/Right pan"
            } else if execution.attachment == RawAttachmentState::Detached {
                "Esc back | / find output | r reattach | c cancel | 1/2 stream | f follow | PgUp/PgDn"
            } else {
                "Esc back | / find output | c cancel | d detach | 1/2 stream | f follow | PgUp/PgDn"
            }
        }
    }
}
