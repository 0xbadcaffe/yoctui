//! Bounded host clipboard reads; never executed for native PTY key input.
use super::*;
use tokio::io::AsyncReadExt;

pub(crate) async fn read_clipboard_command(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String> {
    let mut child = tokio::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .context("clipboard stdout unavailable")?;
    let limit = yoctui_model::TEXTAREA_MAX_PASTE_BYTES;
    let result = tokio::time::timeout(timeout, async {
        let mut bytes = Vec::new();
        stdout
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .await?;
        anyhow::ensure!(
            bytes.len() <= limit,
            "clipboard exceeds the 256 KiB paste limit"
        );
        anyhow::ensure!(child.wait().await?.success(), "clipboard tool failed");
        String::from_utf8(bytes).context("clipboard is not UTF-8 text")
    })
    .await;
    match result {
        Ok(Ok(text)) => Ok(text),
        result => {
            let _ = child.kill().await;
            match result {
                Ok(Err(error)) => Err(error),
                Err(_) => anyhow::bail!("clipboard read timed out"),
                Ok(Ok(_)) => unreachable!(),
            }
        }
    }
}

pub(crate) async fn read_system_clipboard() -> Result<String> {
    let candidates: [(&str, &[&str]); 3] = [
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
        ("xsel", &["--clipboard", "--output"]),
    ];
    let mut failures = Vec::new();
    for (program, args) in candidates {
        match read_clipboard_command(program, args, Duration::from_millis(750)).await {
            Ok(text) => return Ok(text),
            Err(error) => failures.push(format!("{program}: {error}")),
        }
    }
    anyhow::bail!(
        "install wl-paste, xclip or xsel and provide a desktop clipboard; terminal paste also works ({})",
        failures.join("; ")
    )
}
