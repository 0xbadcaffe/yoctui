use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SstateCacheSettings {
    pub sstate_directory: String,
    pub downloads_directory: String,
    pub mirror_url: String,
    pub hash_server: String,
    pub hash_upstream: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SstateCacheProfile {
    pub build_directory: PathBuf,
    pub settings: SstateCacheSettings,
}
impl SstateCacheProfile {
    pub fn review_rows(&self, width: usize) -> Vec<String> {
        use unicode_width::UnicodeWidthChar;
        let keep = |s: &str| if s.is_empty() { "keep existing" } else { s }.to_owned();
        let s = &self.settings;
        let text = format!(
            "Build: {}\n\nSSTATE_DIR: {}\nDL_DIR: {}\nMirror: {}\nHash server: {}\nHash upstream: {}\n\nSaved for future confirmed image/recipe builds in this directory.\nBlank fields preserve current settings. local.conf is unchanged.\nExisting and running builds are unaffected.\nCredentials: configure .netrc on the daemon host.",
            self.build_directory.display(),
            keep(&s.sstate_directory),
            keep(&s.downloads_directory),
            keep(&s.mirror_url),
            keep(&s.hash_server),
            keep(&s.hash_upstream)
        );
        let mut rows = Vec::new();
        for line in text.lines() {
            let mut row = String::new();
            let mut columns = 0;
            for c in line.chars() {
                let cells = c.width().unwrap_or(0);
                if columns + cells > width.max(1) && !row.is_empty() {
                    rows.push(std::mem::take(&mut row));
                    columns = 0;
                }
                row.push(c);
                columns += cells;
            }
            rows.push(row);
        }
        rows
    }
    pub fn validate(&self) -> Result<(), String> {
        if !yoctui_utils::is_absolute_normal_path_within(&self.build_directory, 2048)
            || self.build_directory == std::path::Path::new("/")
        {
            return Err(
                "Cache profiles require an absolute build directory without parent components"
                    .into(),
            );
        }
        self.settings.validate()
    }
}
fn clean(value: &str) -> bool {
    value.len() <= 2048
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '$' | '"' | '\\' | '`'))
}
fn endpoint(value: &str, schemes: &[&str]) -> bool {
    if value.contains(['@', '?', '#', ';', ' ']) || !value.is_ascii() {
        return false;
    }
    let Some(rest) = schemes.iter().find_map(|scheme| value.strip_prefix(scheme)) else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or_default();
    let port = |p: &str| p.parse::<u16>().is_ok_and(|p| p > 0);
    if let Some(ipv6) = host.strip_prefix('[') {
        let Some((address, tail)) = ipv6.split_once(']') else {
            return false;
        };
        return address.parse::<std::net::Ipv6Addr>().is_ok()
            && (tail.is_empty() || tail.strip_prefix(':').is_some_and(port));
    }
    let (name, suffix) = host
        .split_once(':')
        .map_or((host, None), |(n, p)| (n, Some(p)));
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        && suffix.is_none_or(port)
}
fn directory(value: &str) -> bool {
    let path = std::path::Path::new(value);
    value.is_empty()
        || (path != std::path::Path::new("/")
            && yoctui_utils::is_absolute_normal_path_within(path, 2048))
}
impl SstateCacheSettings {
    pub fn validate(&self) -> Result<(), String> {
        let fields = [
            &self.sstate_directory,
            &self.downloads_directory,
            &self.mirror_url,
            &self.hash_server,
            &self.hash_upstream,
        ];
        if fields.iter().any(|s| !clean(s)) {
            return Err("Cache fields must be single-line literal values up to 2048 bytes, without shell/BitBake expansions".into());
        }
        if !directory(&self.sstate_directory) || !directory(&self.downloads_directory) {
            return Err("Cache/download directories must be absolute daemon-host paths without parent components".into());
        }
        if !self.mirror_url.is_empty()
            && !(endpoint(&self.mirror_url, &["https://", "http://"])
                || self.mirror_url.strip_prefix("file://").is_some_and(|s| {
                    !s.is_empty() && directory(s) && !s.contains(['@', '?', '#', ';', ' '])
                }))
        {
            return Err("Mirror URL must be http(s)://host/path or file:///absolute/path; use .netrc for credentials".into());
        }
        if !self.mirror_url.is_empty()
            && self.mirror_url.split('/').filter(|s| *s == "PATH").count()
                > usize::from(self.mirror_url.ends_with("/PATH"))
        {
            return Err("PATH may appear only once at the end of a mirror template".into());
        }
        let hash = |s: &str| {
            endpoint(s, &["wss://", "ws://"])
                || endpoint(&format!("tcp://{s}"), &["tcp://"])
                    && !s.contains('/')
                    && s.rsplit_once(':')
                        .is_some_and(|(_, p)| p.parse::<u16>().is_ok_and(|p| p > 0))
                || s.strip_prefix("unix://")
                    .is_some_and(|s| !s.is_empty() && directory(s))
        };
        if !self.hash_server.is_empty() && self.hash_server != "auto" && !hash(&self.hash_server) {
            return Err(
                "Hash server must be auto, host:port, ws(s)://host/path or unix:///socket".into(),
            );
        }
        if !self.hash_upstream.is_empty()
            && (self.hash_server != "auto" || !hash(&self.hash_upstream))
        {
            return Err("Hash upstream requires hash_server = auto and a valid endpoint".into());
        }
        if fields.iter().all(|s| s.is_empty()) {
            return Err("Configure at least one cache, mirror or hash-equivalence setting".into());
        }
        Ok(())
    }
    pub fn summary(&self) -> String {
        format!(
            "Sstate reuse: mirror {} · hash {} · cache/download paths {}",
            if self.mirror_url.is_empty() {
                "unchanged"
            } else {
                "configured"
            },
            if self.hash_server.is_empty() {
                "unchanged"
            } else {
                &self.hash_server
            },
            if self.sstate_directory.is_empty() && self.downloads_directory.is_empty() {
                "unchanged"
            } else {
                "configured"
            }
        )
    }
    pub(crate) fn document(&self) -> String {
        let mut s = "# Blank fields preserve existing settings. Paths are on the daemon host.\n# Mirror base URL or /PATH template; credentials belong in .netrc.\n".to_owned();
        for (key, value) in [
            ("sstate_directory", &self.sstate_directory),
            ("downloads_directory", &self.downloads_directory),
            ("mirror_url", &self.mirror_url),
            ("hash_server", &self.hash_server),
            ("hash_upstream", &self.hash_upstream),
        ] {
            s.push_str(&crate::popup_toml_document(key, value, None));
        }
        s
    }
    pub(crate) fn parse(document: &str) -> Result<Self, String> {
        let fields = crate::popup_toml_fields(document)?;
        if fields.len() != 5 {
            return Err("Use exactly the five cache configuration fields shown".into());
        }
        let field = |key| {
            fields
                .get(key)
                .cloned()
                .ok_or_else(|| format!("Missing {key}"))
        };
        let settings = Self {
            sstate_directory: field("sstate_directory")?,
            downloads_directory: field("downloads_directory")?,
            mirror_url: field("mirror_url")?,
            hash_server: field("hash_server")?,
            hash_upstream: field("hash_upstream")?,
        };
        settings.validate()?;
        Ok(settings)
    }
}
