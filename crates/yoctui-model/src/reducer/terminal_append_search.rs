//! State transitions beginning with TerminalAppendSearch.
use super::*;

mod append_command_palette_query_to_open_keymap_preferences;
mod close_keymap_preferences_to_backspace_keymap_capture;
mod terminal_append_search_to_select_command_palette;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::TerminalAppendSearch(..)
        | Action::TerminalBackspaceSearch
        | Action::TerminalFinishSearch
        | Action::TerminalClearSearch
        | Action::TerminalStagePaste(..)
        | Action::TerminalConfirmPaste
        | Action::TerminalBeginRename
        | Action::TerminalAppendRename(..)
        | Action::TerminalBackspaceRename
        | Action::TerminalConfirmRename
        | Action::TerminalScroll { .. }
        | Action::TerminalBeginKill
        | Action::TerminalConfirmKill
        | Action::TerminalCancelMode
        | Action::TerminalToggleHelp
        | Action::ResizeFocusedPane { .. }
        | Action::ActivateNavigator
        | Action::Security(..)
        | Action::Qa(..)
        | Action::Maintenance(..)
        | Action::Focus(..)
        | Action::OpenCommandPalette
        | Action::OpenGlobalSearch
        | Action::SelectCommandPalette { .. } => {
            terminal_append_search_to_select_command_palette::reduce_actions(app, action)
        }
        Action::AppendCommandPaletteQuery(..)
        | Action::BackspaceCommandPaletteQuery
        | Action::ClearCommandPaletteQuery
        | Action::BeginGlobalContentSearch
        | Action::AppendInternalLogQuery(..)
        | Action::BackspaceInternalLogQuery
        | Action::AppendLogQuery(..)
        | Action::BackspaceLogQuery
        | Action::NextLogMatch
        | Action::PreviousLogMatch
        | Action::AppendMetadataQuery(..)
        | Action::BackspaceMetadataQuery
        | Action::GlobalContentSearchLoaded { .. }
        | Action::GlobalContentSearchFailed { .. }
        | Action::ActivateCommandPalette
        | Action::RestoreGlobalSearchResults
        | Action::CloseCommandPalette
        | Action::OpenApplicationMenu
        | Action::OpenContextMenu
        | Action::SelectMenuGroup { .. }
        | Action::SelectMenuItem { .. }
        | Action::AppendMenuPrefix(..)
        | Action::BackspaceMenuPrefix
        | Action::CloseMenu
        | Action::SelectSetting { .. }
        | Action::ChangeSelectedSetting { .. }
        | Action::ResetPreferences
        | Action::RetrySettingsPersistence
        | Action::OpenKeymapPreferences => {
            append_command_palette_query_to_open_keymap_preferences::reduce_actions(app, action)
        }
        Action::CloseKeymapPreferences
        | Action::SelectKeymapPreference { .. }
        | Action::BeginKeymapPreferenceSearch
        | Action::AppendKeymapPreferenceQuery(..)
        | Action::BackspaceKeymapPreferenceQuery
        | Action::ClearKeymapPreferenceQuery
        | Action::FinishKeymapPreferenceSearch
        | Action::BeginKeymapCapture
        | Action::AppendKeymapCapture(..)
        | Action::BackspaceKeymapCapture => {
            close_keymap_preferences_to_backspace_keymap_capture::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
