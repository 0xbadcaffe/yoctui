pub const MAX_QA_CHECKS: usize = 256;
pub const MAX_QA_SCOPES: usize = 256;
pub const MAX_QA_REPORTS: usize = 256;
pub const MAX_QA_FINDINGS: usize = 16_384;
pub const MAX_QA_METADATA: usize = 128;
pub const MAX_QA_LIMITATIONS: usize = 128;
pub const MAX_QA_REPORT_PATHS: usize = 256;
pub const MAX_QA_SESSIONS: usize = 64;
pub const MAX_QA_SESSION_OUTPUT: usize = 256;
pub const MAX_QA_TEXT_BYTES: usize = 4_096;
pub const MAX_QA_QUERY_BYTES: usize = 512;
pub const MAX_QA_FINGERPRINT_BYTES: usize = 256;
pub const MAX_QA_LAYER_ARGUMENTS: usize = 64;
pub const MAX_QA_COMPATIBLE_SERIES: usize = 64;

fn bounded_text(value: &str) -> bool {
    is_bounded_plain_text(value, MAX_QA_TEXT_BYTES)
}

fn bounded_token(value: &str) -> bool {
    is_bounded_identifier(value, 256)
}

fn bounded_fingerprint(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_QA_FINGERPRINT_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn absolute_normal_path(path: &Path) -> bool {
    is_absolute_normal_path_within(path, MAX_QA_TEXT_BYTES)
}

fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.retain(|path| absolute_normal_path(path));
    paths.sort();
    paths.dedup();
    paths.truncate(MAX_QA_REPORT_PATHS);
    paths
}

fn normalize_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|value| bounded_text(value));
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_QA_LIMITATIONS);
    limitations
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QaScope {
    pub recipe: RecipeIdentity,
}

impl QaScope {
    pub fn new(recipe: RecipeIdentity) -> Result<Self, &'static str> {
        if !bounded_token(&recipe.name) || !absolute_normal_path(&recipe.file) {
            return Err("QA recipe/provider scope is invalid");
        }
        Ok(Self { recipe })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.recipe.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QaView {
    #[default]
    RecipeKernel,
    LayerQa,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerIdentity {
    pub name: String,
    pub root: PathBuf,
}

impl QaLayerIdentity {
    pub fn new(name: String, root: PathBuf) -> Result<Self, &'static str> {
        if !bounded_token(&name) || !absolute_normal_path(&root) {
            return Err("configured QA layer identity is invalid");
        }
        Ok(Self { name, root })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.name.clone(), self.root.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QaFindingScope {
    Recipe(QaScope),
    Layer(QaLayerIdentity),
}

impl QaFindingScope {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Recipe(scope) => scope.is_valid(),
            Self::Layer(layer) => layer.is_valid(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Recipe(scope) => &scope.recipe.name,
            Self::Layer(layer) => &layer.name,
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Recipe(scope) => &scope.recipe.file,
            Self::Layer(layer) => &layer.root,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaExecutableIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
}

impl QaExecutableIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) || byte_size == 0 {
            return Err("QA executable identity is invalid");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.byte_size, self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaLayerRunCapability {
    Available {
        executable: QaExecutableIdentity,
        arguments: Vec<String>,
        report_roots: Vec<PathBuf>,
    },
    Disabled(String),
}

impl QaLayerRunCapability {
    pub fn disabled_reason(&self) -> Option<&str> {
        match self {
            Self::Available { .. } => None,
            Self::Disabled(reason) => Some(reason),
        }
    }

    fn is_valid_for(&self, layer: &QaLayerIdentity) -> bool {
        match self {
            Self::Available {
                executable,
                arguments,
                report_roots,
            } => {
                executable.is_valid()
                    && !arguments.is_empty()
                    && arguments.len() <= MAX_QA_LAYER_ARGUMENTS
                    && arguments.iter().all(|argument| bounded_text(argument))
                    && arguments
                        .iter()
                        .any(|argument| argument == &layer.root.to_string_lossy())
                    && report_roots.iter().all(|path| absolute_normal_path(path))
            }
            Self::Disabled(reason) => bounded_text(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaConfiguredLayerCapability {
    pub check: QaCheckId,
    pub identity: QaLayerIdentity,
    pub compatible_series: Vec<String>,
    pub run: QaLayerRunCapability,
    pub limitations: Vec<String>,
}

impl QaConfiguredLayerCapability {
    pub fn new(
        check: QaCheckId,
        identity: QaLayerIdentity,
        mut compatible_series: Vec<String>,
        run: QaLayerRunCapability,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !check.is_valid()
            || !identity.is_valid()
            || !run.is_valid_for(&identity)
            || compatible_series.iter().any(|value| !bounded_token(value))
        {
            return Err("configured layer QA capability is invalid");
        }
        compatible_series.sort();
        compatible_series.dedup();
        compatible_series.truncate(MAX_QA_COMPATIBLE_SERIES);
        Ok(Self {
            check,
            identity,
            compatible_series,
            run,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.check.clone(),
            self.identity.clone(),
            self.compatible_series.clone(),
            self.run.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_layer: QaLayerIdentity,
    pub layers: Vec<QaConfiguredLayerCapability>,
    pub limitations: Vec<String>,
}

impl QaLayerCapabilitySnapshot {
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        selected_layer: QaLayerIdentity,
        mut layers: Vec<QaConfiguredLayerCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !selected_layer.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || layers.iter().any(|layer| !layer.is_valid())
        {
            return Err("layer QA capability identity is invalid");
        }
        layers.sort_by(|left, right| left.identity.cmp(&right.identity));
        layers.dedup_by(|left, right| left.identity == right.identity);
        layers.truncate(MAX_QA_SCOPES);
        if !layers.iter().any(|layer| layer.identity == selected_layer) {
            return Err("selected layer is not in the configured QA layer inventory");
        }
        Ok(Self {
            release,
            build_directory,
            selected_layer,
            layers,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.release.clone(),
            self.build_directory.clone(),
            self.selected_layer.clone(),
            self.layers.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QaLayerCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<QaLayerCapabilitySnapshot>),
    Partial {
        snapshot: Box<QaLayerCapabilitySnapshot>,
        limitations: Vec<String>,
    },
    Failed(String),
}

impl QaLayerCapability {
    pub fn snapshot(&self) -> Option<&QaLayerCapabilitySnapshot> {
        match self {
            Self::Available(snapshot) | Self::Partial { snapshot, .. } => Some(snapshot),
            Self::NotInspected | Self::Inspecting | Self::Failed(_) => None,
        }
    }
}
