use super::*;

#[tokio::test]
async fn build_shell_bash_completes_a_real_filename_on_tab() {
    let (root, mut spec) = fixture("");
    fs::write(spec.cwd.join("yoctui_tab_completion_marker"), "fixture").unwrap();
    spec.command.executable = "/bin/bash".into();
    spec.command.arguments = vec!["--noprofile".into(), "--norc".into(), "-i".into()];
    let mut runner = PtyRunner::default();
    runner
        .start(
            spec,
            BTreeMap::from([
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("PS1".into(), "yoctui-test> ".into()),
                ("TERM".into(), "xterm-256color".into()),
                ("INPUTRC".into(), "/dev/null".into()),
            ]),
        )
        .await
        .unwrap();
    let client = PtyClientId([12; 16]);
    runner
        .apply_session_action(PtySessionAction::Attach(client))
        .unwrap();
    runner
        .apply_session_action(PtySessionAction::TakeControl {
            client,
            expected_epoch: 0,
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut output = Vec::new();
        while !String::from_utf8_lossy(&output).contains("yoctui-test>") {
            if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap() {
                output.extend(bytes);
            }
        }
    })
    .await
    .unwrap();
    runner
        .input(client, 1, b"printf '%s\\n' yoctui_tab_co\t\r")
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut output = Vec::new();
        loop {
            if let PtyRunnerEvent::Output { bytes, .. } = runner.next_event().await.unwrap() {
                output.extend(bytes);
                let text = yoctui_utils::strip_ansi(&String::from_utf8_lossy(&output));
                if text
                    .lines()
                    .any(|line| line.trim_matches('\r') == "yoctui_tab_completion_marker")
                {
                    break;
                }
            }
        }
    })
    .await
    .expect("Tab did not complete and execute the fixture filename");
    runner.input(client, 1, b"exit\r").await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), collect_until_exit(&mut runner))
        .await
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}
