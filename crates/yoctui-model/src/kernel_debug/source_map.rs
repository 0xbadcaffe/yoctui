//! Typed, read-only source observations tied to one exact symbols file.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDebugSourceMap {
    pub build_dir: PathBuf,
    pub machine: String,
    pub symbols: PathBuf,
    pub kernel_source: PathBuf,
    pub kernel_build: PathBuf,
}

impl KernelDebugSourceMap {
    pub fn arguments(&self, symbols: &Path) -> Result<Vec<String>, String> {
        // Changing the symbols field invalidates, rather than guesses, its map.
        if symbols != self.symbols {
            return Ok(Vec::new());
        }
        let paths = [
            &self.build_dir,
            &self.symbols,
            &self.kernel_source,
            &self.kernel_build,
        ];
        if paths.iter().any(|path| {
            !yoctui_utils::is_absolute_normal_path(path)
                || path.to_str().is_none_or(|text| {
                    text.len() > super::MAX_KERNEL_DEBUG_FIELD_BYTES
                        || !text
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"/._-+".contains(&b))
                })
        }) || self.symbols.parent() != Some(self.kernel_build.as_path())
            || !self
                .kernel_build
                .starts_with(self.build_dir.join("tmp/work"))
            || !self.kernel_source.starts_with(&self.build_dir)
        {
            return Err("Kernel source mapping is outside its observed build or has unsupported path syntax".into());
        }
        let version = self
            .kernel_build
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .ok_or("Kernel source mapping has no version")?;
        let recipe = self
            .kernel_build
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .filter(|name| name.starts_with("linux-"))
            .ok_or("Kernel source mapping has no kernel provider")?;
        Ok(vec![
            "-iex".into(),
            format!(
                "set substitute-path /usr/src/kernel {}",
                self.kernel_source.display()
            ),
            "-iex".into(),
            format!("directory {}", self.kernel_source.display()),
            "-iex".into(),
            format!(
                "set substitute-path /usr/src/debug/{recipe}/{version} {}",
                self.kernel_build.display()
            ),
            "-iex".into(),
            format!("directory {}", self.kernel_build.display()),
        ])
    }
}
