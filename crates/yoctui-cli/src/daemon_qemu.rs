use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use yoctui_bitbake::{QemuCommandSpec, QemuJobRunner, QemuRunnerEvent, QemuRunnerOutputStream};
use yoctui_model::{
    ImageArtifactIdentity, ImageArtifactKind, QemuDisplayMode, QemuLaunchPreview,
    QemuLaunchRequest, QemuNetworkingMode, QemuSerialMode,
};
use yoctui_protocol::daemon::{DaemonQemuRequest, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonQemuEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: QemuRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        session_id: u64,
        exit_code: i32,
    },
    Failed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
        message: String,
    },
    Cancelled {
        job_id: JobId,
        session_id: u64,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        session_id: u64,
        message: String,
    },
}

pub struct DaemonQemuSupervisor {
    next_job_id: u64,
    active: std::collections::HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonQemuEvent>,
    rx: mpsc::UnboundedReceiver<DaemonQemuEvent>,
}
impl Default for DaemonQemuSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: Default::default(),
            tx,
            rx,
        }
    }
}
impl DaemonQemuSupervisor {
    pub fn start(
        &mut self,
        session_id: u64,
        request: DaemonQemuRequest,
        build_directory: String,
        executable: String,
    ) -> Result<JobId, String> {
        let (preview, cwd) = wire_preview(request, executable, build_directory)?;
        let command = QemuCommandSpec::from_preview(&preview).map_err(|e| e.to_string())?;
        let id = JobId(self.next_job_id);
        self.next_job_id += 1;
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = QemuJobRunner::new(cwd);
            if let Err(e) = runner.start(command).await {
                let _ = tx.send(DaemonQemuEvent::Lost {
                    job_id: id,
                    session_id,
                    message: e.to_string(),
                });
                return;
            }
            loop {
                tokio::select! { c=cancel_rx.recv()=>{ if c.is_some(){let _=runner.cancel().await;} }, e=runner.next_event()=>{ let event=match e { Ok(QemuRunnerEvent::Starting)|Ok(QemuRunnerEvent::Started)=>DaemonQemuEvent::Started{job_id:id,session_id}, Ok(QemuRunnerEvent::Output{stream,line,truncated})=>DaemonQemuEvent::Output{job_id:id,session_id,stream,line,truncated}, Ok(QemuRunnerEvent::Completed{exit_code})=>DaemonQemuEvent::Completed{job_id:id,session_id,exit_code}, Ok(QemuRunnerEvent::Failed{message,exit_code})=>DaemonQemuEvent::Failed{job_id:id,session_id,exit_code,message}, Ok(QemuRunnerEvent::Cancelled{forced,exit_code})=>DaemonQemuEvent::Cancelled{job_id:id,session_id,forced,exit_code}, Ok(QemuRunnerEvent::CancellationRejected{message})|Ok(QemuRunnerEvent::Lost{message})=>DaemonQemuEvent::Lost{job_id:id,session_id,message}, Err(e)=>DaemonQemuEvent::Lost{job_id:id,session_id,message:e.to_string()} }; let terminal=matches!(event,DaemonQemuEvent::Completed{..}|DaemonQemuEvent::Failed{..}|DaemonQemuEvent::Cancelled{..}|DaemonQemuEvent::Lost{..}); if tx.send(event).is_err()||terminal{return;} } }
            }
        });
        Ok(id)
    }
    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown QEMU session {session_id}"))?
            .send(())
            .map_err(|_| "QEMU session is no longer active".into())
    }
    pub fn try_event(&mut self) -> Option<DaemonQemuEvent> {
        let e = self.rx.try_recv().ok()?;
        if matches!(
            e,
            DaemonQemuEvent::Completed { .. }
                | DaemonQemuEvent::Failed { .. }
                | DaemonQemuEvent::Cancelled { .. }
                | DaemonQemuEvent::Lost { .. }
        ) {
            let id = match &e {
                DaemonQemuEvent::Completed { session_id, .. }
                | DaemonQemuEvent::Failed { session_id, .. }
                | DaemonQemuEvent::Cancelled { session_id, .. }
                | DaemonQemuEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(e)
    }
}
fn wire_preview(
    w: DaemonQemuRequest,
    executable: String,
    build_directory: String,
) -> Result<(QemuLaunchPreview, PathBuf), String> {
    let kind = match w.artifact_kind.as_str() {
        "RootFilesystem" => ImageArtifactKind::RootFilesystem,
        "Wic" => ImageArtifactKind::Wic,
        _ => return Err("unsupported QEMU artifact kind".into()),
    };
    let req = QemuLaunchRequest {
        machine: w.machine,
        image: ImageArtifactIdentity {
            machine: w.image_machine,
            image: w.image,
            path: w.image_path.into(),
        },
        artifact_kind: kind,
        kernel: w.kernel.map(Into::into),
        rootfs: w.rootfs.map(Into::into),
        networking: match w.networking.as_str() {
            "Slirp" => QemuNetworkingMode::Slirp,
            "Tap" => QemuNetworkingMode::Tap,
            "None" => QemuNetworkingMode::None,
            _ => return Err("invalid networking mode".into()),
        },
        display: match w.display.as_str() {
            "Graphical" => QemuDisplayMode::Graphical,
            "Nographic" => QemuDisplayMode::Nographic,
            _ => return Err("invalid display mode".into()),
        },
        serial: match w.serial.as_str() {
            "Stdio" => QemuSerialMode::Stdio,
            "Telnet" => QemuSerialMode::Telnet,
            "None" => QemuSerialMode::None,
            _ => return Err("invalid serial mode".into()),
        },
        memory_mib: w.memory_mib,
        extra_arguments: w.extra_arguments,
    };
    req.validate().map_err(str::to_owned)?;
    let mut argv = vec![
        PathBuf::from(executable),
        PathBuf::from(&req.machine),
        req.image.path.clone(),
        format!("qemumemory={}", req.memory_mib).into(),
        match req.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "nonetwork",
        }
        .into(),
        match req.display {
            QemuDisplayMode::Graphical => "sdl",
            QemuDisplayMode::Nographic => "nographic",
        }
        .into(),
    ];
    if let Some(p) = &req.kernel {
        argv.push(p.clone())
    }
    if let Some(p) = &req.rootfs {
        argv.push(p.clone())
    }
    match req.serial {
        QemuSerialMode::Stdio => argv.push("serialstdio".into()),
        QemuSerialMode::Telnet => argv.push("serialtelnet".into()),
        QemuSerialMode::None => {}
    }
    argv.extend(req.extra_arguments.iter().map(PathBuf::from));
    Ok((
        QemuLaunchPreview { request: req, argv },
        Path::new(&build_directory).to_path_buf(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_request_reconstructs_closed_qemu_identity() {
        let request = DaemonQemuRequest {
            machine: "qemux86-64".into(),
            image_machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            image_path: "/tmp/core-image-minimal.ext4".into(),
            artifact_kind: "RootFilesystem".into(),
            kernel: None,
            rootfs: None,
            networking: "Slirp".into(),
            display: "Nographic".into(),
            serial: "Stdio".into(),
            memory_mib: 1024,
            extra_arguments: vec!["foo=bar".into()],
        };
        let (preview, cwd) =
            wire_preview(request, "/tmp/runqemu".into(), "/tmp/build".into()).unwrap();
        assert_eq!(preview.request.machine, "qemux86-64");
        assert_eq!(preview.argv[0], PathBuf::from("/tmp/runqemu"));
        assert_eq!(cwd, PathBuf::from("/tmp/build"));
    }
}
