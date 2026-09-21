//! Read-only, bounded Git status probing. No shell and no network access.
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};
use yoctui_model::{SourceGitStatus, SourceGitSummary};

pub async fn inspect_source_git(source: &Path) -> SourceGitStatus {
    match inspect(source).await {
        Ok(status) => SourceGitStatus::Ready(status),
        Err(error) => SourceGitStatus::Unavailable(error),
    }
}
async fn inspect(source: &Path) -> Result<SourceGitSummary, String> {
    let mut command = Command::new("git");
    command
        .args(["--no-optional-locks", "-C"])
        .arg(source)
        .args([
            "status",
            "--porcelain=2",
            "--branch",
            "-z",
            "--untracked-files=normal",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut group = child.id().map(yoctui_utils::ProcessGroupGuard::new);
    let stdout = child.stdout.take().ok_or("Git stdout unavailable")?;
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::new();
        stdout
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("Git status exceeded 1 MiB".into());
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Source is not a readable Git worktree".into());
        }
        parse_git_status(&bytes)
    })
    .await
    .map_err(|_| "Git status timed out".to_owned())?;
    if result.is_ok()
        && let Some(group) = &mut group
    {
        group.disarm();
    }
    result
}
pub fn parse_git_status(bytes: &[u8]) -> Result<SourceGitSummary, String> {
    let mut status = SourceGitSummary::default();
    let mut records = bytes.split(|b| *b == 0);
    while let Some(record) = records.next() {
        if let Some(value) = record.strip_prefix(b"# branch.head ") {
            status.branch = String::from_utf8_lossy(value)
                .chars()
                .filter(|c| !c.is_control())
                .collect();
        } else if let Some(value) = record.strip_prefix(b"# branch.upstream ") {
            status.upstream = Some(String::from_utf8_lossy(value).into());
        } else if let Some(value) = record.strip_prefix(b"# branch.ab ") {
            let text = String::from_utf8_lossy(value);
            let (ahead, behind) = text.split_once(' ').ok_or("Invalid Git tracking counts")?;
            status.ahead = ahead
                .strip_prefix('+')
                .and_then(|n| n.parse().ok())
                .ok_or("Invalid ahead count")?;
            status.behind = behind
                .strip_prefix('-')
                .and_then(|n| n.parse().ok())
                .ok_or("Invalid behind count")?;
        } else if record.starts_with(b"? ") {
            status.untracked += 1;
        } else if record.starts_with(b"u ") {
            status.conflicts += 1;
        } else if record.starts_with(b"1 ") || record.starts_with(b"2 ") {
            let xy = record.get(2..4).ok_or("Invalid Git index state")?;
            status.staged += usize::from(xy[0] != b'.');
            status.unstaged += usize::from(xy[1] != b'.');
            if record[0] == b'2' {
                records.next().ok_or("Missing Git rename origin")?;
            }
        }
    }
    if status.branch.is_empty() {
        return Err("Missing Git branch state".into());
    }
    Ok(status)
}

#[cfg(test)]
#[path = "tests/source_git/mod.rs"]
mod tests;
