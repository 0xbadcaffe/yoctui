//! External editor.
use super::*;

pub(crate) fn editor_path_error(path: &Path) -> Option<String> {
    match path.try_exists() {
        Ok(true) => None,
        Ok(false) => Some(format!(
            "Cannot open {} because the reported path no longer exists.",
            path.display()
        )),
        Err(error) => Some(format!(
            "Cannot inspect the reported path {}: {error}",
            path.display()
        )),
    }
}

pub(crate) fn run_editor_process(
    editor: &std::ffi::OsStr,
    path: &Path,
) -> std::io::Result<std::process::ExitStatus> {
    ProcessCommand::new(editor).arg(path).status()
}

pub(crate) fn editor_exit_error(status: std::process::ExitStatus, path: &str) -> Option<String> {
    (!status.success()).then(|| format!("$EDITOR exited with {status} while opening {path}."))
}

pub(crate) async fn open_in_editor(
    guard: &TerminalGuard,
    app: &mut App,
    path: PathBuf,
    preferred_editor: Option<&str>,
) {
    if let Some(error) = editor_path_error(&path) {
        app.notification = Some(error);
        return;
    }
    let editor = preferred_editor
        .map(Into::into)
        .or_else(|| env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    let path_label = path.display().to_string();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for $EDITOR: {error}"
        ));
        return;
    }
    let editor_result =
        tokio::task::spawn_blocking(move || run_editor_process(&editor, &path)).await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after $EDITOR: {error}"
        ));
    } else if let Ok(Err(error)) = editor_result {
        app.notification = Some(format!("Could not start $EDITOR: {error}"));
    } else if let Ok(Ok(status)) = editor_result
        && let Some(error) = editor_exit_error(status, &path_label)
    {
        app.notification = Some(error);
    } else if let Err(error) = editor_result {
        app.notification = Some(format!("$EDITOR task failed: {error}"));
    }
}

pub(crate) async fn copy_to_clipboard(app: &mut App, content: String) {
    let result = tokio::task::spawn_blocking(move || -> Result<&'static str> {
        let candidates: [(&str, &[&str]); 3] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ];
        for (program, args) in candidates {
            let Ok(mut child) = ProcessCommand::new(program)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            else {
                continue;
            };
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(content.as_bytes())?;
            }
            if child.wait()?.success() {
                return Ok(program);
            }
        }
        anyhow::bail!("install wl-copy, xclip, or xsel and provide a graphical clipboard")
    })
    .await;
    app.notification = Some(match result {
        Ok(Ok(program)) => format!("Content copied with {program}."),
        Ok(Err(error)) => format!("Could not copy content: {error}"),
        Err(error) => format!("Clipboard task failed: {error}"),
    });
}

pub(crate) async fn open_yocto_shell(guard: &TerminalGuard, app: &mut App) {
    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for the Yocto shell: {error}"
        ));
        return;
    }
    let shell_result =
        tokio::task::spawn_blocking(move || ProcessCommand::new(shell).status()).await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after the Yocto shell: {error}"
        ));
    } else if let Ok(Err(error)) = shell_result {
        app.notification = Some(format!("Could not start the Yocto shell: {error}"));
    } else if let Err(error) = shell_result {
        app.notification = Some(format!("Yocto shell task failed: {error}"));
    } else {
        app.notification = Some("Returned from the inherited Yocto shell.".into());
    }
}
