#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapChordState {
    pub scope: Option<KeymapScope>,
    pub sequence: KeySequence,
}

impl Default for KeymapChordState {
    fn default() -> Self {
        Self {
            scope: None,
            sequence: KeySequence(Vec::new()),
        }
    }
}

impl KeymapChordState {
    pub fn clear(&mut self) {
        self.scope = None;
        self.sequence.0.clear();
    }

    pub fn is_pending(&self) -> bool {
        self.scope.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapResolution {
    Activated(OperatorActionId),
    Pending,
    Unmatched,
}

pub fn keymap_scope_for_action(definition: &OperatorActionDefinition) -> KeymapScope {
    match definition.target {
        OperatorActionTarget::Command(crate::CommandId::SelectImage) => {
            KeymapScope::Workspace(WorkspaceDestination::Images)
        }
        OperatorActionTarget::Command(crate::CommandId::BuildSelectedRecipe) => {
            KeymapScope::Workspace(WorkspaceDestination::Recipes)
        }
        OperatorActionTarget::Command(crate::CommandId::EditBbmask) => {
            KeymapScope::Workspace(WorkspaceDestination::Configuration)
        }
        OperatorActionTarget::Command(_) => KeymapScope::Global,
        OperatorActionTarget::Workspace { destination, .. } => KeymapScope::Workspace(destination),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapPreferenceRow {
    pub action_id: OperatorActionId,
    pub scope: KeymapScope,
    pub label: &'static str,
    pub menu_path: Vec<&'static str>,
    pub sequences: Vec<KeySequence>,
    pub custom: bool,
    pub critical: bool,
}

impl KeymapPreferenceRow {
    pub fn binding_label(&self) -> String {
        if self.sequences.is_empty() {
            "unbound".into()
        } else {
            self.sequences
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" / ")
        }
    }

    pub fn state_label(&self) -> &'static str {
        if self.sequences.is_empty() {
            "disabled"
        } else if self.custom {
            "custom"
        } else {
            "default"
        }
    }
}

pub fn keymap_preference_rows(
    preferences: &KeymapPreferences,
    effective: &EffectiveKeymap,
    query: &str,
) -> Vec<KeymapPreferenceRow> {
    let query = query.trim().to_lowercase();
    let mut rows = global_operator_action_definitions()
        .into_iter()
        .map(|definition| {
            let scope = keymap_scope_for_action(&definition);
            let sequences = effective
                .bindings_for_action(definition.id)
                .filter(|binding| binding.scope == scope)
                .map(|binding| binding.sequence.clone())
                .collect::<Vec<_>>();
            let custom = preferences.overrides.iter().any(|binding| {
                binding.action_id == definition.id.as_str() && binding.scope == scope
            });
            KeymapPreferenceRow {
                action_id: definition.id,
                scope,
                label: definition.label,
                menu_path: definition.menu_path,
                sequences,
                custom,
                critical: critical_keymap_action(definition.id.as_str()),
            }
        })
        .filter(|row| {
            query.is_empty()
                || row.action_id.as_str().contains(&query)
                || row.label.to_lowercase().contains(&query)
                || row.scope.to_string().to_lowercase().contains(&query)
                || row
                    .menu_path
                    .iter()
                    .any(|part| part.to_lowercase().contains(&query))
                || row.binding_label().to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (left.scope, left.label, left.action_id.as_str()).cmp(&(
            right.scope,
            right.label,
            right.action_id.as_str(),
        ))
    });
    rows
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapCaptureState {
    pub strokes: Vec<KeyStroke>,
}

impl KeymapCaptureState {
    pub fn sequence(&self) -> Option<KeySequence> {
        KeySequence::new(self.strokes.clone()).ok()
    }

    pub fn label(&self) -> String {
        if self.strokes.is_empty() {
            "<press 1-3 keys>".into()
        } else {
            self.strokes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapPreferencesUiState {
    pub open: bool,
    pub query: String,
    pub searching: bool,
    pub selection: usize,
    pub capture: Option<KeymapCaptureState>,
    pub validation_error: Option<String>,
}

impl KeymapPreferencesUiState {
    pub fn selected_row(
        &self,
        preferences: &KeymapPreferences,
        effective: &EffectiveKeymap,
    ) -> Option<KeymapPreferenceRow> {
        keymap_preference_rows(preferences, effective, &self.query)
            .get(self.selection)
            .cloned()
    }

    pub fn clamp_selection(
        &mut self,
        preferences: &KeymapPreferences,
        effective: &EffectiveKeymap,
    ) {
        self.selection = self.selection.min(
            keymap_preference_rows(preferences, effective, &self.query)
                .len()
                .saturating_sub(1),
        );
    }
}
