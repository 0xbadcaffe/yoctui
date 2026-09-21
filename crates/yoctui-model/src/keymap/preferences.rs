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
