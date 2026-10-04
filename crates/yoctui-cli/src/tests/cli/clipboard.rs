use super::*;

async fn run(script: &str) -> Result<String> {
    crate::clipboard::read_clipboard_command("/bin/sh", &["-c", script], Duration::from_secs(1))
        .await
}

#[tokio::test]
async fn clipboard_reader_preserves_utf8_newlines_and_bounds_process_output() {
    assert_eq!(
        run("printf 'UTF-8 猫\\n\\n'").await.unwrap(),
        "UTF-8 猫\n\n"
    );
    assert!(run("exit 3").await.is_err());
    assert!(run("printf '\\377'").await.is_err());
    assert!(run("head -c 262145 /dev/zero").await.is_err());
    assert!(
        crate::clipboard::read_clipboard_command(
            "/no/such/clipboard",
            &[],
            Duration::from_millis(20)
        )
        .await
        .is_err()
    );
    assert!(
        crate::clipboard::read_clipboard_command(
            "/bin/sh",
            &["-c", "exec sleep 10"],
            Duration::from_millis(20)
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("timed out")
    );
}

#[test]
fn clipboard_ctrl_v_decodes_without_changing_native_key_bytes() {
    let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(input_from_key(key), Some(Input::CtrlV));
    assert_eq!(terminal_key_bytes(key, false).unwrap(), b"\x16");
}
