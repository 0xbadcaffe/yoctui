//! Termination.
use super::*;

#[cfg(unix)]
pub(crate) fn termination_receiver() -> Result<tokio::sync::mpsc::Receiver<()>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::spawn(async move {
        sigterm.recv().await;
        let _ = sender.send(()).await;
    });
    Ok(receiver)
}

#[cfg(unix)]
pub(crate) fn termination_requested(receiver: &mut tokio::sync::mpsc::Receiver<()>) -> bool {
    receiver.try_recv().is_ok()
}
