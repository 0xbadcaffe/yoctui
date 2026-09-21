#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QaCheckFamily {
    KernelConfiguration,
    UriFetch,
    Patch,
    License,
    RecipePackage,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaCheckId(pub String);

impl QaCheckId {
    pub fn new(value: String) -> Result<Self, &'static str> {
        if !bounded_token(&value) {
            return Err("QA check identity is invalid");
        }
        Ok(Self(value))
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.0.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaCheckAvailability {
    Available,
    Disabled(String),
}

impl QaCheckAvailability {
    pub fn disabled_reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Disabled(reason) => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaCheckCapability {
    pub id: QaCheckId,
    pub family: QaCheckFamily,
    pub label: String,
    pub scope: QaScope,
    pub task: Option<String>,
    pub report_roots: Vec<PathBuf>,
    pub availability: QaCheckAvailability,
    pub limitations: Vec<String>,
}

impl QaCheckCapability {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: QaCheckId,
        family: QaCheckFamily,
        label: String,
        scope: QaScope,
        task: Option<String>,
        report_roots: Vec<PathBuf>,
        availability: QaCheckAvailability,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !id.is_valid()
            || !bounded_text(&label)
            || !scope.is_valid()
            || task.as_deref().is_some_and(|value| !bounded_token(value))
            || report_roots.iter().any(|path| !absolute_normal_path(path))
            || matches!(
                &availability,
                QaCheckAvailability::Disabled(reason) if !bounded_text(reason)
            )
        {
            return Err("QA check capability is invalid");
        }
        let report_roots = normalize_paths(report_roots);
        match &availability {
            QaCheckAvailability::Available if task.is_none() => {
                return Err("available QA check has no capability-supplied task");
            }
            QaCheckAvailability::Disabled(_) if task.is_some() => {
                return Err("disabled QA check cannot expose an executable task");
            }
            _ => {}
        }
        Ok(Self {
            id,
            family,
            label,
            scope,
            task,
            report_roots,
            availability,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.id.clone(),
            self.family,
            self.label.clone(),
            self.scope.clone(),
            self.task.clone(),
            self.report_roots.clone(),
            self.availability.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_scope: QaScope,
    pub scopes: Vec<QaScope>,
    pub checks: Vec<QaCheckCapability>,
    pub limitations: Vec<String>,
}

impl QaCapabilitySnapshot {
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        selected_scope: QaScope,
        mut scopes: Vec<QaScope>,
        mut checks: Vec<QaCheckCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !selected_scope.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || scopes.iter().any(|scope| !scope.is_valid())
            || checks.iter().any(|check| !check.is_valid())
        {
            return Err("QA capability identity is invalid");
        }
        if !scopes.contains(&selected_scope) {
            scopes.insert(0, selected_scope.clone());
        }
        scopes.dedup();
        scopes.truncate(MAX_QA_SCOPES);
        if checks.iter().any(|check| !scopes.contains(&check.scope)) {
            return Err("QA check scope is not capability supplied");
        }
        checks.sort_by(|left, right| {
            left.scope
                .recipe
                .name
                .cmp(&right.scope.recipe.name)
                .then_with(|| left.scope.recipe.file.cmp(&right.scope.recipe.file))
                .then_with(|| left.id.cmp(&right.id))
        });
        checks.dedup_by(|left, right| left.scope == right.scope && left.id == right.id);
        checks.truncate(MAX_QA_CHECKS);
        Ok(Self {
            release,
            build_directory,
            selected_scope,
            scopes,
            checks,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.release.clone(),
            self.build_directory.clone(),
            self.selected_scope.clone(),
            self.scopes.clone(),
            self.checks.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QaCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<QaCapabilitySnapshot>),
    Partial {
        snapshot: Box<QaCapabilitySnapshot>,
        limitations: Vec<String>,
    },
    Failed(String),
}

impl QaCapability {
    pub fn snapshot(&self) -> Option<&QaCapabilitySnapshot> {
        match self {
            Self::Available(snapshot) | Self::Partial { snapshot, .. } => Some(snapshot),
            Self::NotInspected | Self::Inspecting | Self::Failed(_) => None,
        }
    }
}
