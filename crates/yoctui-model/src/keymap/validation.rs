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
