pub const MAX_SECURITY_REPORTS: usize = 256;
pub const MAX_SECURITY_FINDINGS: usize = 16_384;
pub const MAX_SECURITY_COMPONENTS: usize = 16_384;
pub const MAX_SECURITY_METADATA: usize = 128;
pub const MAX_SECURITY_LIMITATIONS: usize = 128;
pub const MAX_SECURITY_TEXT_BYTES: usize = 4_096;
pub const MAX_SECURITY_QUERY_BYTES: usize = 512;
pub const MAX_SECURITY_FINGERPRINT_BYTES: usize = 256;
pub const MAX_SECURITY_PATHS: usize = 256;
pub const MAX_SECURITY_SESSIONS: usize = 64;
pub const MAX_SECURITY_SESSION_OUTPUT: usize = 256;

fn bounded_text(value: &str) -> bool {
    is_bounded_plain_text(value, MAX_SECURITY_TEXT_BYTES)
}

fn bounded_token(value: &str) -> bool {
    is_bounded_identifier(value, 256)
}

fn absolute_normal_path(path: &Path) -> bool {
    is_absolute_normal_path_within(path, 4_096)
}

fn bounded_fingerprint(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SECURITY_FINGERPRINT_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn normalize_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|value| bounded_text(value));
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_SECURITY_LIMITATIONS);
    limitations
}

fn normalize_metadata(mut metadata: Vec<SecurityMetadata>) -> Vec<SecurityMetadata> {
    metadata.retain(SecurityMetadata::is_valid);
    metadata.sort();
    metadata.dedup();
    metadata.truncate(MAX_SECURITY_METADATA);
    metadata
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SecurityView {
    #[default]
    Cves,
    Sbom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecurityScope {
    Recipe(RecipeIdentity),
    Image {
        target: String,
        machine: String,
        distro: String,
    },
}

impl SecurityScope {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Recipe(identity) => {
                bounded_token(&identity.name) && absolute_normal_path(&identity.file)
            }
            Self::Image {
                target,
                machine,
                distro,
            } => bounded_token(target) && bounded_token(machine) && bounded_token(distro),
        }
    }

    pub fn target(&self) -> &str {
        match self {
            Self::Recipe(identity) => &identity.name,
            Self::Image { target, .. } => target,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityMapperCapability {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

impl SecurityMapperCapability {
    pub fn is_valid(&self) -> bool {
        absolute_normal_path(&self.executable)
            && self.arguments.len() <= 64
            && self.arguments.iter().all(|argument| bounded_text(argument))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub scope: SecurityScope,
    pub available_scopes: Vec<SecurityScope>,
    pub cve_task: Option<String>,
    pub recipe_sbom_task: Option<String>,
    pub image_sbom_task: Option<String>,
    pub image_build_emits_sbom: bool,
    pub mapper: Option<SecurityMapperCapability>,
    pub cve_roots: Vec<PathBuf>,
    pub sbom_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

impl SecurityCapabilitySnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        scope: SecurityScope,
        mut available_scopes: Vec<SecurityScope>,
        cve_task: Option<String>,
        recipe_sbom_task: Option<String>,
        image_sbom_task: Option<String>,
        image_build_emits_sbom: bool,
        mapper: Option<SecurityMapperCapability>,
        mut cve_roots: Vec<PathBuf>,
        mut sbom_roots: Vec<PathBuf>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !scope.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || cve_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || recipe_sbom_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || image_sbom_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || mapper.as_ref().is_some_and(|value| !value.is_valid())
        {
            return Err("security capability identity is invalid");
        }
        cve_roots.retain(|path| absolute_normal_path(path));
        cve_roots.sort();
        cve_roots.dedup();
        cve_roots.truncate(MAX_SECURITY_PATHS);
        sbom_roots.retain(|path| absolute_normal_path(path));
        sbom_roots.sort();
        sbom_roots.dedup();
        sbom_roots.truncate(MAX_SECURITY_PATHS);
        available_scopes.retain(SecurityScope::is_valid);
        if !available_scopes.contains(&scope) {
            available_scopes.insert(0, scope.clone());
        }
        available_scopes.dedup();
        available_scopes.truncate(MAX_SECURITY_PATHS);
        Ok(Self {
            release,
            build_directory,
            scope,
            available_scopes,
            cve_task,
            recipe_sbom_task,
            image_sbom_task,
            image_build_emits_sbom,
            mapper,
            cve_roots,
            sbom_roots,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn cve_unavailable_reason(&self) -> Option<&'static str> {
        self.cve_task
            .is_none()
            .then_some("do_cve_check is not reported for the exact scope")
    }

    pub fn sbom_unavailable_reason(&self) -> Option<&'static str> {
        (self.recipe_sbom_task.is_none()
            && self.image_sbom_task.is_none()
            && !self.image_build_emits_sbom)
            .then_some("no authoritative SBOM task or image-build capability is reported")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SecurityCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<SecurityCapabilitySnapshot>),
    Failed(String),
}
