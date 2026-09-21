#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectProfileError {
    #[error("unsupported project profile schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid project profile field `{field}`: {reason}")]
    InvalidField { field: String, reason: String },
    #[error("duplicate `{name}` in {collection}")]
    DuplicateName {
        collection: &'static str,
        name: String,
    },
    #[error("workflow references unknown build preset `{0}`")]
    UnknownPreset(String),
}

fn bounded_collection(field: &'static str, len: usize) -> Result<(), ProjectProfileError> {
    if len > MAX_COLLECTION_ITEMS {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: format!("contains more than {MAX_COLLECTION_ITEMS} entries"),
        });
    }
    Ok(())
}

fn validate_identities(field: &str, values: &[String]) -> Result<(), ProjectProfileError> {
    bounded_collection(
        match field {
            "favorites.recipes" => "favorites.recipes",
            "favorites.images" => "favorites.images",
            "favorites.layers" => "favorites.layers",
            _ => "profile identities",
        },
        values.len(),
    )?;
    let mut unique = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        validate_identity(&format!("{field}[{index}]"), value)?;
        if !unique.insert(value) {
            return Err(ProjectProfileError::InvalidField {
                field: format!("{field}[{index}]"),
                reason: "duplicate identity".into(),
            });
        }
    }
    Ok(())
}

fn validate_identity(field: &str, value: &str) -> Result<(), ProjectProfileError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(|character| {
        character.is_control()
            || character.is_whitespace()
            || !matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '+' | '/')
    }) {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: "must be a bounded portable logical identity".into(),
        });
    }
    Ok(())
}

fn validate_label(field: &str, value: &str) -> Result<(), ProjectProfileError> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > MAX_LABEL_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: "must be a nonempty bounded label without control characters".into(),
        });
    }
    Ok(())
}

fn validate_portable_path(value: &str) -> Result<(), ProjectProfileError> {
    let invalid = value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."));
    if invalid {
        return Err(ProjectProfileError::InvalidField {
            field: "project path".into(),
            reason: "must be a portable repository-relative path without escape components".into(),
        });
    }
    Ok(())
}
