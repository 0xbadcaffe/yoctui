impl Default for DevtoolInspector {
    fn default() -> Self {
        Self {
            devtool_program: None,
            git_program: "git".into(),
        }
    }
}

impl DevtoolInspector {
    pub fn with_programs(devtool_program: PathBuf, git_program: PathBuf) -> Self {
        Self {
            devtool_program: Some(devtool_program),
            git_program,
        }
    }

    pub async fn inspect(&self, _build_dir: &Path, identity: RecipeIdentity) -> DevtoolStatus {
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Unavailable {
                reason: "Devtool status requires the current environment capability snapshot."
                    .into(),
            },
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        }
    }

    pub async fn inspect_with_compatibility(
        &self,
        build_dir: &Path,
        identity: RecipeIdentity,
        compatibility: &yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> DevtoolStatus {
        if !identity.file.is_absolute() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: Some(DevtoolStatusError::InvalidRecipeIdentity),
            };
        }

        let executable = self.devtool_program.clone().or_else(|| {
            compatibility
                .snapshot
                .environment
                .available_tools
                .value()
                .and_then(|tools| tools.iter().find(|tool| tool.id == "devtool"))
                .map(|tool| tool.executable.clone())
        });
        let command = executable
            .ok_or(DevtoolCompatibilityError::ToolIdentityUnknown)
            .and_then(|executable| {
                DevtoolCommandPlanner::new(
                    compatibility,
                    expected_generation,
                    build_dir,
                    &executable,
                )?
                .status()
            });
        let command = match command {
            Ok(command) => command,
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Unavailable {
                        reason: error.to_string(),
                    },
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: None,
                };
            }
        };
        let output = TokioCommand::new(command.executable())
            .args(command.arguments())
            .current_dir(build_dir)
            .output()
            .await;
        let output = match output {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::MissingExecutable,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: None,
                };
            }
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::DevtoolFailed {
                        exit_code: None,
                        message: error.to_string(),
                    }),
                };
            }
        };
        if !output.status.success() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: Some(DevtoolStatusError::DevtoolFailed {
                    exit_code: output.status.code(),
                    message: output_text(&output.stderr),
                }),
            };
        }
        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(error) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::MalformedOutput {
                        line: error.to_string(),
                    }),
                };
            }
        };
        let entries = match parse_devtool_status(&stdout) {
            Ok(entries) => entries,
            Err(line) => {
                return DevtoolStatus {
                    identity,
                    capability: DevtoolCapability::Available,
                    workspace: DevtoolWorkspace::NotMember,
                    git: DevtoolGitState::NotApplicable,
                    error: Some(DevtoolStatusError::MalformedOutput { line }),
                };
            }
        };
        let Some((source_path, recipe_file)) = entries
            .into_iter()
            .find(|(recipe, _, _)| recipe == &identity.name)
            .map(|(_, source_path, recipe_file)| (source_path, recipe_file))
        else {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: DevtoolGitState::NotApplicable,
                error: None,
            };
        };
        if !source_path.is_dir() {
            return DevtoolStatus {
                identity,
                capability: DevtoolCapability::Available,
                workspace: DevtoolWorkspace::MissingDirectory { source_path },
                git: DevtoolGitState::NotApplicable,
                error: None,
            };
        }
        let git = inspect_git(&self.git_program, &source_path).await;
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path,
                recipe_file,
            },
            git,
            error: None,
        }
    }
}

pub(crate) fn parse_devtool_status(
    output: &str,
) -> Result<Vec<(String, PathBuf, Option<PathBuf>)>, String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !["NOTE: ", "WARNING: ", "DEBUG: "]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
        })
        .map(|line| {
            let (recipe, value) = line.split_once(": ").ok_or_else(|| line.to_owned())?;
            if recipe.is_empty() || value.is_empty() {
                return Err(line.to_owned());
            }
            let (source, recipe_file) = value
                .strip_suffix(')')
                .and_then(|value| value.rsplit_once(" ("))
                .map_or((value, None), |(source, recipe_file)| {
                    (source, Some(PathBuf::from(recipe_file)))
                });
            let source = PathBuf::from(source);
            if !source.is_absolute() {
                return Err(line.to_owned());
            }
            Ok((recipe.to_owned(), source, recipe_file))
        })
        .collect()
}

pub(crate) async fn inspect_git(program: &Path, source_path: &Path) -> DevtoolGitState {
    let output = TokioCommand::new(program)
        .arg("-C")
        .arg(source_path)
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .await;
    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DevtoolGitState::MissingExecutable;
        }
        Err(error) => {
            return DevtoolGitState::Failed {
                exit_code: None,
                message: error.to_string(),
            };
        }
    };
    if !output.status.success() {
        let message = output_text(&output.stderr);
        if message
            .to_ascii_lowercase()
            .contains("not a git repository")
        {
            return DevtoolGitState::NotRepository;
        }
        return DevtoolGitState::Failed {
            exit_code: output.status.code(),
            message,
        };
    }
    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(error) => {
            return DevtoolGitState::Malformed {
                message: error.to_string(),
            };
        }
    };
    parse_git_status(&output).unwrap_or_else(|message| DevtoolGitState::Malformed { message })
}

pub(crate) fn parse_git_status(output: &str) -> Result<DevtoolGitState, String> {
    let mut branch = None;
    let mut head = None;
    let mut modified = 0;
    let mut untracked = 0;
    let mut conflicted = 0;
    for line in output.lines().filter(|line| !line.is_empty()) {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            branch = (value != "(detached)").then(|| value.to_owned());
        } else if let Some(value) = line.strip_prefix("# branch.oid ") {
            head = (value != "(initial)").then(|| value.to_owned());
        } else if line.starts_with("# branch.") {
            continue;
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            modified += 1;
        } else if line.starts_with("u ") {
            conflicted += 1;
        } else if line.starts_with("? ") {
            untracked += 1;
        } else if line.starts_with("! ") {
            continue;
        } else {
            return Err(format!("unrecognized Git status record: {line}"));
        }
    }
    Ok(DevtoolGitState::Available {
        branch,
        head,
        modified,
        untracked,
        conflicted,
    })
}
