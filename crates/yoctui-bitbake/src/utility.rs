use std::path::{Path, PathBuf};

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
}
