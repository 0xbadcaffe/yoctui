use std::path::{Path, PathBuf};

use thiserror::Error;
use yoctui_model::{CapabilityId, DaemonCompatibilitySnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilityRisk {
    ReadOnly,
    Mutating,
    Network,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityCommandSpec {
    pub executable: PathBuf,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub environment: Vec<(String, String)>,
    pub risk: UtilityRisk,
    pub timeout_seconds: u64,
}

impl UtilityCommandSpec {
    pub fn new(
        executable: impl Into<PathBuf>,
        cwd: impl Into<PathBuf>,
        argv: Vec<String>,
        risk: UtilityRisk,
    ) -> Result<Self, String> {
        let executable = executable.into();
        let cwd = cwd.into();
        if executable.as_os_str().is_empty() || !cwd.is_absolute() {
            return Err("executable and absolute cwd are required".into());
        }
        if argv
            .iter()
            .any(|arg| arg.is_empty() || arg.chars().any(|c| c.is_control()))
        {
            return Err("arguments must be non-empty and free of control bytes".into());
        }
        Ok(Self {
            executable,
            argv,
            cwd,
            environment: Vec::new(),
            risk,
            timeout_seconds: 3600,
        })
    }

    pub fn indexed_argv(&self) -> Vec<String> {
        std::iter::once(self.executable.display().to_string())
            .chain(self.argv.iter().cloned())
            .enumerate()
            .map(|(i, value)| format!("[{i}] {value}"))
            .collect()
    }

    pub fn validate_cwd(&self, build_dir: &Path) -> Result<(), String> {
        self.cwd
            .starts_with(build_dir)
            .then_some(())
            .ok_or_else(|| "utility cwd must remain inside the configured build directory".into())
    }
}

/// Exact environment authority for an expert or less-specialized utility
/// command. Specialized adapters may add stronger argument validation, but
/// must enforce the same snapshot/tool/implementation tuple before spawn.
pub struct UtilityCompatibilityAuthority<'a> {
    authority: &'a DaemonCompatibilitySnapshot,
    build_directory: &'a Path,
    executable: &'a Path,
}

impl<'a> UtilityCompatibilityAuthority<'a> {
    pub fn new(
        authority: &'a DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &'a Path,
        tool_id: &str,
    ) -> Result<Self, UtilityCompatibilityError> {
        if authority.snapshot.generation != expected_generation {
            return Err(UtilityCompatibilityError::StaleGeneration {
                expected: expected_generation,
                actual: authority.snapshot.generation,
            });
        }
        if authority
            .snapshot
            .environment
            .build_directory
            .value()
            .map(PathBuf::as_path)
            != Some(build_directory)
        {
            return Err(UtilityCompatibilityError::EnvironmentMismatch);
        }
        let tool = authority
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == tool_id))
            .ok_or_else(|| UtilityCompatibilityError::ToolIdentityUnknown(tool_id.into()))?;
        Ok(Self {
            authority,
            build_directory,
            executable: &tool.executable,
        })
    }

    pub fn command(
        &self,
        capability: CapabilityId,
        expected_implementation: &str,
        argv: Vec<String>,
        risk: UtilityRisk,
    ) -> Result<UtilityCommandSpec, UtilityCompatibilityError> {
        let record = self
            .authority
            .snapshot
            .capability(capability)
            .ok_or(UtilityCompatibilityError::MissingCapability(capability))?;
        if !record.state.is_enabled() {
            return Err(UtilityCompatibilityError::Unavailable {
                capability,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| "capability is not enabled".into()),
            });
        }
        let selected = self
            .authority
            .implementations
            .get(&capability)
            .ok_or(UtilityCompatibilityError::MissingImplementation(capability))?;
        if selected.id != expected_implementation {
            return Err(UtilityCompatibilityError::ImplementationMismatch {
                capability,
                expected: expected_implementation.into(),
                actual: selected.id.clone(),
            });
        }
        UtilityCommandSpec::new(self.executable, self.build_directory, argv, risk)
            .map_err(UtilityCompatibilityError::InvalidCommand)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UtilityCompatibilityError {
    #[error("stale capability snapshot: expected generation {expected}, received {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("capability snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("utility tool identity is unknown for {0}")]
    ToolIdentityUnknown(String),
    #[error("capability snapshot has no record for {0}")]
    MissingCapability(CapabilityId),
    #[error("{capability} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("capability snapshot has no selected implementation for {0}")]
    MissingImplementation(CapabilityId),
    #[error(
        "selected implementation for {capability} is {actual}, but this command requires {expected}"
    )]
    ImplementationMismatch {
        capability: CapabilityId,
        expected: String,
        actual: String,
    },
    #[error("invalid utility command: {0}")]
    InvalidCommand(String),
}

/// Parse expert arguments without invoking a shell. Quotes group whitespace;
/// backslash escapes the following ordinary character.
pub fn parse_utility_arguments(input: &str) -> Result<Vec<String>, String> {
    if input
        .chars()
        .any(|c| c == '\0' || c.is_control() && c != ' ')
    {
        return Err("arguments contain NUL/control bytes".into());
    }
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escaped || quote.is_some() {
        return Err("unterminated quote or escape".into());
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
    };

    fn authority(generation: u64, build: &Path, executable: &Path) -> DaemonCompatibilitySnapshot {
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "runqemu".into(),
                            executable: executable.to_owned(),
                            version: None,
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: vec![CapabilityRecord {
                    id: CapabilityId::RunQemu,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: "runqemu --help".into(),
                        detail: "The initialized runqemu executable accepted help.".into(),
                        argv: vec![executable.display().to_string(), "--help".into()],
                    }],
                }],
            },
            implementations: BTreeMap::from([(
                CapabilityId::RunQemu,
                CapabilityImplementation {
                    id: "runqemu.argv".into(),
                    kind: CapabilityImplementationKind::Command,
                },
            )]),
        }
        .normalize()
        .unwrap()
    }
    #[test]
    fn utility_runner_parses_argv_and_rejects_shell_controls() {
        assert_eq!(
            parse_utility_arguments("--name 'core image' --flag").unwrap(),
            vec!["--name", "core image", "--flag"]
        );
        assert!(parse_utility_arguments("echo; rm -rf /").is_ok());
        assert!(parse_utility_arguments("unterminated\"").is_err());
    }
    #[test]
    fn utility_runner_preview_is_indexed_and_cwd_bounded() {
        let spec = UtilityCommandSpec::new(
            "/usr/bin/bitbake",
            "/tmp/build",
            vec!["core-image-minimal".into()],
            UtilityRisk::ReadOnly,
        )
        .unwrap();
        assert_eq!(spec.indexed_argv()[1], "[1] core-image-minimal");
        assert!(spec.validate_cwd(Path::new("/tmp/build")).is_ok());
        assert!(spec.validate_cwd(Path::new("/tmp/other")).is_err());
    }

    #[test]
    fn compatibility_utilities_authorizes_exact_snapshot_tool_and_implementation() {
        let authority = authority(9, Path::new("/yocto/build"), Path::new("/poky/runqemu"));
        let planner =
            UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "runqemu")
                .unwrap();
        let command = planner
            .command(
                CapabilityId::RunQemu,
                "runqemu.argv",
                vec!["qemux86-64".into()],
                UtilityRisk::Mutating,
            )
            .unwrap();
        assert_eq!(command.executable, Path::new("/poky/runqemu"));
        assert_eq!(command.argv, ["qemux86-64"]);
        assert_eq!(command.cwd, Path::new("/yocto/build"));
    }

    #[test]
    fn compatibility_utilities_rejects_stale_unknown_and_wrong_implementation() {
        let authority = authority(9, Path::new("/yocto/build"), Path::new("/poky/runqemu"));
        assert!(matches!(
            UtilityCompatibilityAuthority::new(&authority, 8, Path::new("/yocto/build"), "runqemu"),
            Err(UtilityCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "wic"),
            Err(UtilityCompatibilityError::ToolIdentityUnknown(_))
        ));
        let planner =
            UtilityCompatibilityAuthority::new(&authority, 9, Path::new("/yocto/build"), "runqemu")
                .unwrap();
        assert!(matches!(
            planner.command(
                CapabilityId::RunQemu,
                "runqemu.legacy",
                vec!["qemux86-64".into()],
                UtilityRisk::Mutating
            ),
            Err(UtilityCompatibilityError::ImplementationMismatch { .. })
        ));
    }
}
