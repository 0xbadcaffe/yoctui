//! Termination.
use super::*;

#[cfg(unix)]
pub(crate) fn termination_receiver() -> Result<tokio::sync::mpsc::Receiver<()>> {
    termination_receiver_for(&[SignalKind::terminate()])
}

#[cfg(unix)]
pub(crate) fn interactive_termination_receiver() -> Result<tokio::sync::mpsc::Receiver<()>> {
    termination_receiver_for(&[
        SignalKind::terminate(),
        SignalKind::hangup(),
        SignalKind::interrupt(),
    ])
}

#[cfg(unix)]
fn termination_receiver_for(kinds: &[SignalKind]) -> Result<tokio::sync::mpsc::Receiver<()>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    for kind in kinds {
        let mut incoming = signal(*kind)?;
        let sender = sender.clone();
        tokio::spawn(async move {
            if incoming.recv().await.is_some() {
                let _ = sender.send(()).await;
            }
        });
    }
    Ok(receiver)
}

#[cfg(unix)]
pub(crate) fn termination_requested(receiver: &mut tokio::sync::mpsc::Receiver<()>) -> bool {
    receiver.try_recv().is_ok()
}
