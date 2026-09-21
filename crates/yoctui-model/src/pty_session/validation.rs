fn validate_spec(spec: &PtySessionSpec) -> Result<(), PtySessionError> {
    if spec.id.0 == 0 {
        return Err(PtySessionError::InvalidSessionId);
    }
    validate_name(&spec.name)?;
    spec.dimensions.validate()?;
    for path in [
        &spec.cwd,
        &spec.command.executable,
        &spec.workspace.source_dir,
        &spec.workspace.build_dir,
    ]
    .into_iter()
    .chain(spec.workspace.authorized_context_roots.iter())
    {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(PtySessionError::InvalidPath(path.clone()));
        }
    }
    if !spec.cwd.starts_with(&spec.workspace.source_dir)
        && !spec.cwd.starts_with(&spec.workspace.build_dir)
        && !spec
            .workspace
            .authorized_context_roots
            .iter()
            .any(|root| spec.cwd.starts_with(root))
    {
        return Err(PtySessionError::CwdOutsideWorkspace);
    }
    if spec.workspace.owner_identity.trim().is_empty()
        || spec.workspace.owner_identity.len() > 512
        || spec.workspace.owner_identity.chars().any(char::is_control)
    {
        return Err(PtySessionError::InvalidWorkspaceIdentity);
    }
    if spec.command.arguments.len() > MAX_PTY_ARGUMENTS
        || spec
            .command
            .arguments
            .iter()
            .map(String::len)
            .sum::<usize>()
            > MAX_PTY_ARGUMENT_BYTES
        || spec
            .command
            .arguments
            .iter()
            .any(|argument| argument.contains('\0'))
    {
        return Err(PtySessionError::InvalidCommand);
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), PtySessionError> {
    if name.trim().is_empty()
        || name.len() > MAX_PTY_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(PtySessionError::InvalidName);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PtySessionError {
    #[error("PTY session ID must be nonzero")]
    InvalidSessionId,
    #[error("PTY session name is empty, oversized, or contains controls")]
    InvalidName,
    #[error("invalid PTY dimensions: {0:?}")]
    InvalidDimensions(PtyDimensions),
    #[error("invalid PTY path: {0}")]
    InvalidPath(PathBuf),
    #[error("PTY working directory is outside its workspace")]
    CwdOutsideWorkspace,
    #[error("invalid PTY workspace identity")]
    InvalidWorkspaceIdentity,
    #[error("PTY command arguments are invalid or oversized")]
    InvalidCommand,
    #[error("invalid PTY process group: {0}")]
    InvalidProcessGroup(i32),
    #[error("invalid PTY transition from {0:?}")]
    InvalidTransition(PtySessionLifecycle),
    #[error("PTY is not running: {0:?}")]
    NotRunning(PtySessionLifecycle),
    #[error("PTY client is not attached: {0:?}")]
    ClientNotAttached(PtyClientId),
    #[error("PTY already has a writer")]
    WriterBusy,
    #[error("PTY action does not own the writer lease")]
    NotWriter,
    #[error("stale PTY writer epoch {actual}; expected {expected}")]
    StaleWriterEpoch { expected: u64, actual: u64 },
    #[error("PTY writer epoch is exhausted")]
    WriterEpochExhausted,
    #[error("invalid PTY scrollback metadata")]
    InvalidScrollback,
}
