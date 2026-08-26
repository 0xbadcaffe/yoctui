use std::{collections::HashMap, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{
    OperatorActionDefinition, OperatorActionId, OperatorActionTarget, WorkspaceDestination,
    global_operator_action_definitions,
};

pub const KEYMAP_SCHEMA_VERSION: u16 = 1;
pub const MAX_KEYMAP_OVERRIDES: usize = 256;
pub const MAX_BINDINGS_PER_ACTION: usize = 8;
pub const MAX_KEY_SEQUENCE_STROKES: usize = 3;
pub const MAX_EFFECTIVE_KEYMAP_REPORT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyStroke {
    Char(char),
    Esc,
    Enter,
    Backspace,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    CtrlB,
    CtrlC,
    CtrlP,
    CtrlS,
    CtrlU,
    CtrlV,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
}

impl KeyStroke {
    pub const fn is_terminal_prefix(self) -> bool {
        matches!(self, Self::CtrlB)
    }
}

impl fmt::Display for KeyStroke {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Char(character) => return write!(formatter, "{character}"),
            Self::Esc => "Esc",
            Self::Enter => "Enter",
            Self::Backspace => "Backspace",
            Self::Tab => "Tab",
            Self::BackTab => "Shift+Tab",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Home => "Home",
            Self::End => "End",
            Self::CtrlB => "Ctrl+B",
            Self::CtrlC => "Ctrl+C",
            Self::CtrlP => "Ctrl+P",
            Self::CtrlS => "Ctrl+S",
            Self::CtrlU => "Ctrl+U",
            Self::CtrlV => "Ctrl+V",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
        };
        formatter.write_str(name)
    }
}

impl FromStr for KeyStroke {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let named = match value.to_ascii_lowercase().as_str() {
            "esc" | "escape" => Some(Self::Esc),
            "enter" | "return" => Some(Self::Enter),
            "backspace" => Some(Self::Backspace),
            "tab" => Some(Self::Tab),
            "shift+tab" | "backtab" => Some(Self::BackTab),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "home" => Some(Self::Home),
            "end" => Some(Self::End),
            "ctrl+b" => Some(Self::CtrlB),
            "ctrl+c" => Some(Self::CtrlC),
            "ctrl+p" => Some(Self::CtrlP),
            "ctrl+s" => Some(Self::CtrlS),
            "ctrl+u" => Some(Self::CtrlU),
            "ctrl+v" => Some(Self::CtrlV),
            "f1" => Some(Self::F1),
            "f2" => Some(Self::F2),
            "f3" => Some(Self::F3),
            "f4" => Some(Self::F4),
            "f5" => Some(Self::F5),
            "f6" => Some(Self::F6),
            "f7" => Some(Self::F7),
            "f8" => Some(Self::F8),
            "f9" => Some(Self::F9),
            "f10" => Some(Self::F10),
            _ => None,
        };
        if let Some(stroke) = named {
            return Ok(stroke);
        }
        let mut characters = value.chars();
        let Some(character) = characters.next() else {
            return Err(KeymapError::InvalidStroke(value.into()));
        };
        if characters.next().is_some() || character.is_control() || character.is_whitespace() {
            return Err(KeymapError::InvalidStroke(value.into()));
        }
        Ok(Self::Char(character))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeySequence(Vec<KeyStroke>);

impl KeySequence {
    pub fn new(strokes: Vec<KeyStroke>) -> Result<Self, KeymapError> {
        if strokes.is_empty() || strokes.len() > MAX_KEY_SEQUENCE_STROKES {
            return Err(KeymapError::InvalidSequenceLength(strokes.len()));
        }
        Ok(Self(strokes))
    }

    pub fn single(stroke: KeyStroke) -> Self {
        Self(vec![stroke])
    }

    pub fn strokes(&self) -> &[KeyStroke] {
        &self.0
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn pushed(&self, stroke: KeyStroke) -> Result<Self, KeymapError> {
        let mut strokes = self.0.clone();
        strokes.push(stroke);
        Self::new(strokes)
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, stroke) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(" ")?;
            }
            write!(formatter, "{stroke}")?;
        }
        Ok(())
    }
}

impl FromStr for KeySequence {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let strokes = value
            .split_ascii_whitespace()
            .map(KeyStroke::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(strokes)
    }
}

impl Serialize for KeySequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeySequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(tag = "kind", content = "workspace", rename_all = "snake_case")]
pub enum KeymapScope {
    #[default]
    Global,
    Workspace(WorkspaceDestination),
}

impl fmt::Display for KeymapScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => formatter.write_str("Global"),
            Self::Workspace(destination) => write!(formatter, "{} workspace", destination.label()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapOverride {
    #[serde(alias = "action")]
    pub action_id: String,
    #[serde(default)]
    pub scope: KeymapScope,
    #[serde(default, alias = "keys")]
    pub sequences: Vec<KeySequence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapPreferences {
    #[serde(default)]
    pub schema_version: u16,
    #[serde(default, alias = "bindings")]
    pub overrides: Vec<KeymapOverride>,
}

impl Default for KeymapPreferences {
    fn default() -> Self {
        Self {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: Vec::new(),
        }
    }
}

impl KeymapPreferences {
    pub fn migrate(mut self) -> Result<Self, KeymapError> {
        match self.schema_version {
            0 => self.schema_version = KEYMAP_SCHEMA_VERSION,
            KEYMAP_SCHEMA_VERSION => {}
            version => return Err(KeymapError::UnsupportedSchema(version)),
        }
        EffectiveKeymap::from_preferences(&self)?;
        Ok(self)
    }

    pub fn with_action_sequences(
        &self,
        action_id: OperatorActionId,
        scope: KeymapScope,
        sequences: Vec<KeySequence>,
    ) -> Self {
        let mut next = self.clone();
        next.overrides
            .retain(|binding| binding.action_id != action_id.as_str() || binding.scope != scope);
        next.overrides.push(KeymapOverride {
            action_id: action_id.as_str().into(),
            scope,
            sequences,
        });
        next
    }

    pub fn reset_action(&self, action_id: OperatorActionId, scope: KeymapScope) -> Self {
        let mut next = self.clone();
        next.overrides
            .retain(|binding| binding.action_id != action_id.as_str() || binding.scope != scope);
        next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveKeyBinding {
    pub action_id: OperatorActionId,
    pub scope: KeymapScope,
    pub sequence: KeySequence,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveKeymap {
    pub schema_version: u16,
    bindings: Vec<EffectiveKeyBinding>,
}

impl Default for EffectiveKeymap {
    fn default() -> Self {
        Self::from_preferences(&KeymapPreferences::default()).expect("the built-in keymap is valid")
    }
}

impl EffectiveKeymap {
    pub fn from_preferences(preferences: &KeymapPreferences) -> Result<Self, KeymapError> {
        if preferences.schema_version != KEYMAP_SCHEMA_VERSION {
            return Err(KeymapError::UnsupportedSchema(preferences.schema_version));
        }
        if preferences.overrides.len() > MAX_KEYMAP_OVERRIDES {
            return Err(KeymapError::TooManyOverrides(preferences.overrides.len()));
        }

        let definitions = global_operator_action_definitions();
        let by_id = definitions
            .iter()
            .map(|definition| (definition.id.as_str(), definition))
            .collect::<HashMap<_, _>>();
        let mut overrides = HashMap::new();
        for binding in &preferences.overrides {
            let Some(definition) = by_id.get(binding.action_id.as_str()) else {
                return Err(KeymapError::UnknownAction(binding.action_id.clone()));
            };
            let expected_scope = keymap_scope_for_action(definition);
            if binding.scope != expected_scope {
                return Err(KeymapError::ScopeMismatch {
                    action: binding.action_id.clone(),
                    expected: expected_scope,
                    actual: binding.scope,
                });
            }
            if binding.sequences.len() > MAX_BINDINGS_PER_ACTION {
                return Err(KeymapError::TooManyBindings {
                    action: binding.action_id.clone(),
                    count: binding.sequences.len(),
                });
            }
            if overrides
                .insert((binding.action_id.as_str(), binding.scope), binding)
                .is_some()
            {
                return Err(KeymapError::DuplicateOverride {
                    action: binding.action_id.clone(),
                    scope: binding.scope,
                });
            }
        }

        let mut bindings = Vec::new();
        for definition in &definitions {
            let scope = keymap_scope_for_action(definition);
            if let Some(custom) = overrides.get(&(definition.id.as_str(), scope)) {
                for sequence in &custom.sequences {
                    bindings.push(EffectiveKeyBinding {
                        action_id: definition.id,
                        scope,
                        sequence: sequence.clone(),
                        is_default: false,
                    });
                }
            } else {
                for sequence in &definition.default_bindings {
                    bindings.push(EffectiveKeyBinding {
                        action_id: definition.id,
                        scope,
                        sequence: sequence.parse::<KeySequence>().map_err(|error| {
                            KeymapError::InvalidDefault {
                                action: definition.id.as_str().into(),
                                sequence: (*sequence).into(),
                                reason: error.to_string(),
                            }
                        })?,
                        is_default: true,
                    });
                }
            }
        }

        validate_effective_bindings(&bindings)?;
        validate_critical_reachability(&bindings)?;
        Ok(Self {
            schema_version: KEYMAP_SCHEMA_VERSION,
            bindings,
        })
    }

    pub fn bindings(&self) -> &[EffectiveKeyBinding] {
        &self.bindings
    }

    pub fn bindings_for_action(
        &self,
        action_id: OperatorActionId,
    ) -> impl Iterator<Item = &EffectiveKeyBinding> {
        self.bindings
            .iter()
            .filter(move |binding| binding.action_id == action_id)
    }

    pub fn report(&self) -> String {
        let mut bindings = self.bindings.clone();
        bindings.sort_by(|left, right| {
            (left.scope, left.action_id.as_str(), &left.sequence).cmp(&(
                right.scope,
                right.action_id.as_str(),
                &right.sequence,
            ))
        });
        let mut report = format!("yoctui keymap schema {}\n", self.schema_version);
        for binding in bindings {
            report.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                binding.scope,
                binding.action_id.as_str(),
                binding.sequence,
                if binding.is_default {
                    "default"
                } else {
                    "custom"
                }
            ));
        }
        debug_assert!(report.len() <= MAX_EFFECTIVE_KEYMAP_REPORT_BYTES);
        report
    }

    pub fn resolve_input(
        &self,
        state: &mut KeymapChordState,
        workspace: WorkspaceDestination,
        stroke: KeyStroke,
    ) -> KeymapResolution {
        if let Some(scope) = state.scope {
            let candidate = state
                .sequence
                .pushed(stroke)
                .unwrap_or_else(|_| KeySequence::single(stroke));
            match self.resolve_scope(scope, &candidate) {
                KeymapResolution::Unmatched => {
                    state.clear();
                    return self.resolve_fresh(state, workspace, stroke);
                }
                KeymapResolution::Pending => {
                    state.sequence = candidate;
                    return KeymapResolution::Pending;
                }
                KeymapResolution::Activated(action) => {
                    state.clear();
                    return KeymapResolution::Activated(action);
                }
            }
        }
        self.resolve_fresh(state, workspace, stroke)
    }

    fn resolve_fresh(
        &self,
        state: &mut KeymapChordState,
        workspace: WorkspaceDestination,
        stroke: KeyStroke,
    ) -> KeymapResolution {
        let sequence = KeySequence::single(stroke);
        for scope in [KeymapScope::Workspace(workspace), KeymapScope::Global] {
            match self.resolve_scope(scope, &sequence) {
                KeymapResolution::Unmatched => {}
                KeymapResolution::Pending => {
                    state.scope = Some(scope);
                    state.sequence = sequence;
                    return KeymapResolution::Pending;
                }
                activated => return activated,
            }
        }
        KeymapResolution::Unmatched
    }

    fn resolve_scope(&self, scope: KeymapScope, sequence: &KeySequence) -> KeymapResolution {
        let mut prefix = false;
        for binding in self
            .bindings
            .iter()
            .filter(|binding| binding.scope == scope)
        {
            if &binding.sequence == sequence {
                return KeymapResolution::Activated(binding.action_id);
            }
            prefix |= binding.sequence.starts_with(sequence);
        }
        if prefix {
            KeymapResolution::Pending
        } else {
            KeymapResolution::Unmatched
        }
    }
}

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

fn validate_effective_bindings(bindings: &[EffectiveKeyBinding]) -> Result<(), KeymapError> {
    for (index, left) in bindings.iter().enumerate() {
        if left.sequence.strokes().first().is_some_and(|stroke| {
            stroke.is_terminal_prefix()
                && matches!(
                    left.scope,
                    KeymapScope::Global
                        | KeymapScope::Workspace(WorkspaceDestination::TerminalSessions)
                )
        }) {
            return Err(KeymapError::ReservedTerminalPrefix {
                action: left.action_id.as_str().into(),
                scope: left.scope,
                sequence: left.sequence.clone(),
            });
        }
        for right in &bindings[index + 1..] {
            if left.scope == right.scope
                && (left.sequence.starts_with(&right.sequence)
                    || right.sequence.starts_with(&left.sequence))
            {
                return Err(KeymapError::Collision {
                    scope: left.scope,
                    first_action: left.action_id.as_str().into(),
                    first_sequence: left.sequence.clone(),
                    second_action: right.action_id.as_str().into(),
                    second_sequence: right.sequence.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_critical_reachability(bindings: &[EffectiveKeyBinding]) -> Result<(), KeymapError> {
    for action in ["help.open", "navigate.dashboard"] {
        if !bindings
            .iter()
            .any(|binding| binding.action_id.as_str() == action)
        {
            return Err(KeymapError::UnreachableCriticalAction(action.into()));
        }
    }
    Ok(())
}

pub fn critical_keymap_action(action_id: &str) -> bool {
    matches!(action_id, "help.open" | "navigate.dashboard")
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeymapError {
    #[error("unsupported keymap schema version {0}")]
    UnsupportedSchema(u16),
    #[error("invalid key stroke {0:?}")]
    InvalidStroke(String),
    #[error("key sequence contains {0} strokes; expected 1..={MAX_KEY_SEQUENCE_STROKES}")]
    InvalidSequenceLength(usize),
    #[error("keymap contains {0} overrides; maximum is {MAX_KEYMAP_OVERRIDES}")]
    TooManyOverrides(usize),
    #[error("unknown operator action {0:?}")]
    UnknownAction(String),
    #[error("action {action:?} belongs to {expected}, not {actual}")]
    ScopeMismatch {
        action: String,
        expected: KeymapScope,
        actual: KeymapScope,
    },
    #[error("action {action:?} has {count} bindings; maximum is {MAX_BINDINGS_PER_ACTION}")]
    TooManyBindings { action: String, count: usize },
    #[error("action {action:?} has more than one override in {scope}")]
    DuplicateOverride { action: String, scope: KeymapScope },
    #[error("invalid built-in binding {sequence:?} for {action:?}: {reason}")]
    InvalidDefault {
        action: String,
        sequence: String,
        reason: String,
    },
    #[error(
        "keymap collision in {scope}: {first_action} ({first_sequence}) conflicts with {second_action} ({second_sequence})"
    )]
    Collision {
        scope: KeymapScope,
        first_action: String,
        first_sequence: KeySequence,
        second_action: String,
        second_sequence: KeySequence,
    },
    #[error("{action} cannot bind reserved terminal prefix {sequence} in {scope}")]
    ReservedTerminalPrefix {
        action: String,
        scope: KeymapScope,
        sequence: KeySequence,
    },
    #[error("critical action {0} has no reachable binding")]
    UnreachableCriticalAction(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn override_binding(action_id: &str, scope: KeymapScope, sequences: &[&str]) -> KeymapOverride {
        KeymapOverride {
            action_id: action_id.into(),
            scope,
            sequences: sequences
                .iter()
                .map(|sequence| sequence.parse().unwrap())
                .collect(),
        }
    }

    #[test]
    fn ux_keymap_defaults_are_valid_scoped_and_exportable() {
        let keymap = EffectiveKeymap::default();
        assert!(keymap.bindings.len() >= 20);
        assert!(keymap.report().starts_with("yoctui keymap schema 1\n"));
        let select_image = keymap
            .bindings
            .iter()
            .find(|binding| binding.action_id.as_str() == "navigate.select-image")
            .unwrap();
        assert_eq!(
            select_image.scope,
            KeymapScope::Workspace(WorkspaceDestination::Images)
        );
        assert_eq!(select_image.sequence.to_string(), "i");
        assert!(keymap.bindings.iter().any(|binding| {
            binding.action_id.as_str() == "configure.bbmask"
                && binding.sequence.to_string() == "x e"
        }));
    }

    #[test]
    fn ux_keymap_resolves_workspace_before_global_and_bounded_chords() {
        let keymap = EffectiveKeymap::default();
        let mut state = KeymapChordState::default();
        assert_eq!(
            keymap.resolve_input(
                &mut state,
                WorkspaceDestination::Images,
                KeyStroke::Char('i')
            ),
            KeymapResolution::Activated(OperatorActionId::new("navigate.select-image"))
        );
        assert_eq!(
            keymap.resolve_input(
                &mut state,
                WorkspaceDestination::Tasks,
                KeyStroke::Char('i')
            ),
            KeymapResolution::Activated(OperatorActionId::new("navigate.images"))
        );
        assert_eq!(
            keymap.resolve_input(
                &mut state,
                WorkspaceDestination::Configuration,
                KeyStroke::Char('x')
            ),
            KeymapResolution::Pending
        );
        assert!(state.is_pending());
        assert_eq!(
            keymap.resolve_input(
                &mut state,
                WorkspaceDestination::Configuration,
                KeyStroke::Char('e')
            ),
            KeymapResolution::Activated(OperatorActionId::new("configure.bbmask"))
        );
        assert!(!state.is_pending());
    }

    #[test]
    fn ux_keymap_rejects_collision_prefix_reserved_unknown_scope_and_unreachable() {
        let collision = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding(
                "navigate.logs",
                KeymapScope::Global,
                &["e"],
            )],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&collision),
            Err(KeymapError::Collision { .. })
        ));

        let prefix = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding(
                "navigate.logs",
                KeymapScope::Global,
                &["Esc x"],
            )],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&prefix),
            Err(KeymapError::Collision { .. })
        ));

        let reserved = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding(
                "navigate.logs",
                KeymapScope::Global,
                &["Ctrl+B l"],
            )],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&reserved),
            Err(KeymapError::ReservedTerminalPrefix { .. })
        ));

        let unknown = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding(
                "missing.action",
                KeymapScope::Global,
                &["z"],
            )],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&unknown),
            Err(KeymapError::UnknownAction(_))
        ));

        let wrong_scope = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding(
                "navigate.logs",
                KeymapScope::Workspace(WorkspaceDestination::Logs),
                &["z"],
            )],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&wrong_scope),
            Err(KeymapError::ScopeMismatch { .. })
        ));

        let unreachable = KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION,
            overrides: vec![override_binding("help.open", KeymapScope::Global, &[])],
        };
        assert!(matches!(
            EffectiveKeymap::from_preferences(&unreachable),
            Err(KeymapError::UnreachableCriticalAction(action)) if action == "help.open"
        ));
    }

    #[test]
    fn ux_keymap_migrates_legacy_alias_fields_and_rejects_future_schema() {
        let legacy: KeymapPreferences = toml::from_str(
            "schema_version = 0\n[[bindings]]\naction = 'navigate.logs'\nkeys = ['z', 'g l']\n",
        )
        .unwrap();
        let migrated = legacy.migrate().unwrap();
        assert_eq!(migrated.schema_version, KEYMAP_SCHEMA_VERSION);
        let effective = EffectiveKeymap::from_preferences(&migrated).unwrap();
        assert_eq!(
            effective
                .bindings_for_action(OperatorActionId::new("navigate.logs"))
                .map(|binding| binding.sequence.to_string())
                .collect::<Vec<_>>(),
            ["z", "g l"]
        );

        assert!(matches!(
            KeymapPreferences {
                schema_version: KEYMAP_SCHEMA_VERSION + 1,
                overrides: Vec::new(),
            }
            .migrate(),
            Err(KeymapError::UnsupportedSchema(_))
        ));
    }
}
