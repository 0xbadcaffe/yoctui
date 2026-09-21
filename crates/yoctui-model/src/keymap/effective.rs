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
