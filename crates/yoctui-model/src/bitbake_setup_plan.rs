use crate::{TerminalCreationKind, TerminalLaunchRequest};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeSetupPlan {
    pub tool: PathBuf,
    pub top_dir: PathBuf,
    pub name: String,
    pub template: String,
    pub registry: String,
}

pub(crate) fn validate_top(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path.parent().is_none()
        || path.file_name().is_none()
        || path.components().any(|c| matches!(c, Component::ParentDir))
        || value.len() > 4096
        || value.chars().any(char::is_control)
    {
        return Err(
            "Choose an absolute top directory (not /), with no '..' or control characters.".into(),
        );
    }
    Ok(())
}

impl BitBakeSetupPlan {
    pub fn from_values(values: &[String; 5]) -> Result<Self, String> {
        if values
            .iter()
            .any(|v| v.len() > 4096 || v.chars().any(char::is_control))
        {
            return Err("Fields must be single-line text up to 4096 bytes.".into());
        }
        validate_top(&values[1])?;
        let tool = PathBuf::from(&values[0]);
        if tool.file_name().is_none_or(|name| name != "bitbake-setup")
            || (!tool.is_absolute() && values[0] != "bitbake-setup")
        {
            return Err("Choose bitbake-setup from PATH or its absolute executable path.".into());
        }
        let name = &values[2];
        if name.is_empty()
            || name.len() > 80
            || name.starts_with('-')
            || matches!(name.as_str(), "." | "..")
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(
                "Setup name must be one directory name using letters, digits, '.', '-' or '_'."
                    .into(),
            );
        }
        if values[3].starts_with('-') {
            return Err("Template must be an ID, file path or URL, not an option.".into());
        }
        Ok(Self {
            tool,
            top_dir: values[1].clone().into(),
            name: name.clone(),
            template: values[3].clone(),
            registry: values[4].clone(),
        })
    }

    pub fn destination(&self) -> PathBuf {
        self.top_dir.join(&self.name)
    }

    pub fn launch(&self) -> Result<TerminalLaunchRequest, String> {
        Self::from_values(&[
            self.tool.display().to_string(),
            self.top_dir.display().to_string(),
            self.name.clone(),
            self.template.clone(),
            self.registry.clone(),
        ])?;
        let mut arguments = vec![
            "--setting".into(),
            "default".into(),
            "top-dir-prefix".into(),
            self.top_dir.parent().unwrap().display().to_string(),
            "--setting".into(),
            "default".into(),
            "top-dir-name".into(),
            self.top_dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ];
        if !self.registry.is_empty() {
            arguments.extend([
                "--setting".into(),
                "default".into(),
                "registry".into(),
                self.registry.clone(),
            ]);
        }
        arguments.extend([
            "init".into(),
            "--no-init-vscode".into(),
            "--setup-dir-name".into(),
            self.name.clone(),
        ]);
        if !self.template.is_empty() {
            arguments.push(self.template.clone());
        }
        Ok(TerminalLaunchRequest {
            name: format!("BitBake setup: {}", self.name),
            kind: TerminalCreationKind::Utility,
            cwd: self.top_dir.parent().unwrap().into(),
            program: self.tool.clone(),
            arguments,
            completion: None,
        })
    }
}
