use super::*;

#[tokio::test]
async fn oversized_process_line_is_truncated_and_stream_continues() {
    let (mut writer, reader) = tokio::io::duplex(MAX_PROCESS_LINE_BYTES + 2);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let reader_task = tokio::spawn(read_output(reader, sender));
    writer
        .write_all(&vec![b'x'; MAX_PROCESS_LINE_BYTES + 1])
        .await
        .unwrap();
    writer.write_all(b"\nnext line\n").await.unwrap();
    drop(writer);
    reader_task.await.unwrap();
    assert!(
        receiver
            .recv()
            .await
            .unwrap()
            .message
            .ends_with("[line truncated]")
    );
    assert_eq!(receiver.recv().await.unwrap().message, "next line");
}
