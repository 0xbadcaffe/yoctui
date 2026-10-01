use std::{
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Child,
    task::JoinHandle,
};

const LIMIT: usize = 4 * 1024 * 1024;
pub(in crate::qemu_debug) fn capture(child: &mut Child, file: File) -> Vec<JoinHandle<()>> {
    let log = Arc::new(Mutex::new((file, 0)));
    vec![
        reader(child.stdout.take().expect("piped stdout"), log.clone()),
        reader(child.stderr.take().expect("piped stderr"), log),
    ]
}
fn reader(
    mut stream: impl AsyncRead + Unpin + Send + 'static,
    log: Arc<Mutex<(File, usize)>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut bytes = [0_u8; 4096];
        while let Ok(count) = stream.read(&mut bytes).await {
            if count == 0 {
                break;
            }
            let Ok(mut log) = log.lock() else {
                break;
            };
            let retained = count.min(LIMIT.saturating_sub(log.1));
            if retained > 0 {
                if log.0.write_all(&bytes[..retained]).is_err() {
                    break;
                }
                log.1 += retained;
            }
            // Keep draining after the shared cap; never block QEMU on a full log.
        }
    })
}
