use super::*;

#[tokio::test]
async fn maintenance_sstate_closed_stdin_does_not_mask_child_outcome() {
    let (mut stdin, child_stdin) = tokio::io::duplex(1);
    drop(child_stdin);

    write_process_stdin(&mut stdin, b"n\n").await.unwrap();
}
