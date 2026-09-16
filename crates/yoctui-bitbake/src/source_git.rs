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
mod tests {
    use super::*;
    fn git(root: &Path, args: &[&str]) {
        let result = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    #[tokio::test]
    async fn git_status_real_worktree_reports_dirty_tracking_and_missing_source() {
        let root = std::env::temp_dir().join(format!("yoctui-git-status-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-b", "master"]);
        std::fs::write(root.join("tracked"), "first\n").unwrap();
        git(&root, &["add", "tracked"]);
        git(&root, &["commit", "-m", "first"]);
        git(&root, &["branch", "upstream"]);
        git(&root, &["branch", "--set-upstream-to=upstream"]);
        let SourceGitStatus::Ready(clean) = inspect_source_git(&root).await else {
            panic!("expected status");
        };
        assert_eq!(
            (clean.ahead, clean.behind, clean.staged, clean.unstaged),
            (0, 0, 0, 0)
        );
        std::fs::write(root.join("tracked"), "second\n").unwrap();
        git(&root, &["commit", "-am", "second"]);
        let SourceGitStatus::Ready(ahead) = inspect_source_git(&root).await else {
            panic!();
        };
        assert_eq!((ahead.ahead, ahead.behind), (1, 0));
        git(&root, &["checkout", "upstream"]);
        git(&root, &["branch", "--set-upstream-to=master"]);
        let SourceGitStatus::Ready(behind) = inspect_source_git(&root).await else {
            panic!();
        };
        assert_eq!((behind.ahead, behind.behind), (0, 1));
        git(&root, &["branch", "--unset-upstream"]);
        std::fs::write(root.join("tracked"), "staged\n").unwrap();
        git(&root, &["add", "tracked"]);
        std::fs::write(root.join("tracked"), "unstaged\n").unwrap();
        std::fs::write(root.join("new\nfile"), "new").unwrap();
        let SourceGitStatus::Ready(dirty) = inspect_source_git(&root).await else {
            panic!();
        };
        assert_eq!((dirty.staged, dirty.unstaged, dirty.untracked), (1, 1, 1));
        assert!(dirty.upstream.is_none());
        std::fs::remove_dir_all(&root).unwrap();
        assert!(matches!(
            inspect_source_git(&root).await,
            SourceGitStatus::Unavailable(_)
        ));
    }
    #[test]
    fn git_status_rename_origin_cannot_spoof_headers() {
        let status = parse_git_status(b"# branch.head master\0# branch.upstream origin/master\0# branch.ab +2 -3\x002 R. details\0# branch.ab +99 -99\0u UU details\0? strange\nname\0").unwrap();
        assert_eq!(
            (
                status.ahead,
                status.behind,
                status.staged,
                status.conflicts,
                status.untracked
            ),
            (2, 3, 1, 1, 1)
        );
        assert!(parse_git_status(b"# branch.ab rubbish\0").is_err());
    }
}
