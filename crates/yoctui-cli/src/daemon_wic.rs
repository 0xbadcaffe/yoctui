use std::path::PathBuf;
use tokio::sync::mpsc;
use yoctui_bitbake::WicDeviceInspector;
use yoctui_bitbake::{WicCreateCommandSpec, WicJobRunner, WicRunnerEvent, WicRunnerOutputStream};
use yoctui_model::{
    WicCapability, WicCompression, WicCreatePreview, WicCreateRequest, WicDeviceIdentity,
    WicKickstart, WicKickstartIdentity, WicOutputIdentity, WicWriteRequest,
};
use yoctui_protocol::daemon::{DaemonWicCreateRequest, JobId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonWicEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: WicRunnerOutputStream,
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
pub struct DaemonWicSupervisor {
    next: u64,
    active: std::collections::HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonWicEvent>,
    rx: mpsc::UnboundedReceiver<DaemonWicEvent>,
}
impl Default for DaemonWicSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next: 1,
            active: Default::default(),
            tx,
            rx,
        }
    }
}
impl DaemonWicSupervisor {
    pub fn start_write(
        &mut self,
        session_id: u64,
        executable: String,
        image_path: String,
        device: WicDeviceIdentity,
        build: String,
    ) -> Result<JobId, String> {
        device.validate().map_err(str::to_owned)?;
        let request = WicWriteRequest {
            executable: executable.into(),
            image: WicOutputIdentity {
                path: image_path.into(),
                size_bytes: 0,
                modified_unix_seconds: 0,
            },
            device,
        };
        let id = JobId(self.next);
        self.next += 1;
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = WicJobRunner::new(build.into());
            let inspector = WicDeviceInspector::default();
            if let Err(e) = runner.start_write(&inspector, request).await {
                let _ = tx.send(DaemonWicEvent::Lost {
                    job_id: id,
                    session_id,
                    message: e.to_string(),
                });
                return;
            }
            loop {
                tokio::select! { c=cancel_rx.recv()=>{if c.is_some(){let _=runner.cancel().await;}}, e=runner.next_event()=>{ let event=match e { Ok(WicRunnerEvent::Starting)|Ok(WicRunnerEvent::Started)=>DaemonWicEvent::Started{job_id:id,session_id}, Ok(WicRunnerEvent::Output{stream,line,truncated})=>DaemonWicEvent::Output{job_id:id,session_id,stream,line,truncated}, Ok(WicRunnerEvent::Completed{exit_code,..})=>DaemonWicEvent::Completed{job_id:id,session_id,exit_code}, Ok(WicRunnerEvent::Failed{message,exit_code})=>DaemonWicEvent::Failed{job_id:id,session_id,exit_code,message}, Ok(WicRunnerEvent::Cancelled{forced,exit_code})=>DaemonWicEvent::Cancelled{job_id:id,session_id,forced,exit_code}, Ok(WicRunnerEvent::CancellationRejected{message})|Ok(WicRunnerEvent::Lost{message})=>DaemonWicEvent::Lost{job_id:id,session_id,message}, Err(e)=>DaemonWicEvent::Lost{job_id:id,session_id,message:e.to_string()} }; let terminal=matches!(event,DaemonWicEvent::Completed{..}|DaemonWicEvent::Failed{..}|DaemonWicEvent::Cancelled{..}|DaemonWicEvent::Lost{..}); if tx.send(event).is_err()||terminal{return;} } }
            }
        });
        Ok(id)
    }
    pub fn start(
        &mut self,
        session_id: u64,
        w: DaemonWicCreateRequest,
        build: String,
        executable: String,
    ) -> Result<JobId, String> {
        let (preview, output) = wire_preview(w, executable)?;
        let capability = WicCapability::Available {
            executable: preview.argv[0].clone(),
            kickstarts: vec![preview.kickstart.clone()],
            image_targets: vec![preview.request.image.clone()],
        };
        let command =
            WicCreateCommandSpec::from_preview(&preview, &capability).map_err(|e| e.to_string())?;
        let id = JobId(self.next);
        self.next += 1;
        let (tx_cancel, mut rx_cancel) = mpsc::unbounded_channel();
        self.active.insert(session_id, tx_cancel);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = WicJobRunner::new(build.into());
            if let Err(e) = runner.start(command, output).await {
                let _ = tx.send(DaemonWicEvent::Lost {
                    job_id: id,
                    session_id,
                    message: e.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {c=rx_cancel.recv()=>{if c.is_some(){let _=runner.cancel().await;}},e=runner.next_event()=>{let event=match e{Ok(WicRunnerEvent::Starting)|Ok(WicRunnerEvent::Started)=>DaemonWicEvent::Started{job_id:id,session_id},Ok(WicRunnerEvent::Output{stream,line,truncated})=>DaemonWicEvent::Output{job_id:id,session_id,stream,line,truncated},Ok(WicRunnerEvent::Completed{exit_code,..})=>DaemonWicEvent::Completed{job_id:id,session_id,exit_code},Ok(WicRunnerEvent::Failed{message,exit_code})=>DaemonWicEvent::Failed{job_id:id,session_id,exit_code,message},Ok(WicRunnerEvent::Cancelled{forced,exit_code})=>DaemonWicEvent::Cancelled{job_id:id,session_id,forced,exit_code},Ok(WicRunnerEvent::CancellationRejected{message})|Ok(WicRunnerEvent::Lost{message})=>DaemonWicEvent::Lost{job_id:id,session_id,message},Err(e)=>DaemonWicEvent::Lost{job_id:id,session_id,message:e.to_string()}};let terminal=matches!(event,DaemonWicEvent::Completed{..}|DaemonWicEvent::Failed{..}|DaemonWicEvent::Cancelled{..}|DaemonWicEvent::Lost{..});if tx.send(event).is_err()||terminal{return;}}}
            }
        });
        Ok(id)
    }
    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown Wic session {session_id}"))?
            .send(())
            .map_err(|_| "Wic session is no longer active".into())
    }
    pub fn try_event(&mut self) -> Option<DaemonWicEvent> {
        let e = self.rx.try_recv().ok()?;
        if matches!(
            e,
            DaemonWicEvent::Completed { .. }
                | DaemonWicEvent::Failed { .. }
                | DaemonWicEvent::Cancelled { .. }
                | DaemonWicEvent::Lost { .. }
        ) {
            let id = match &e {
                DaemonWicEvent::Completed { session_id, .. }
                | DaemonWicEvent::Failed { session_id, .. }
                | DaemonWicEvent::Cancelled { session_id, .. }
                | DaemonWicEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(e)
    }
}
fn wire_preview(
    w: DaemonWicCreateRequest,
    executable: String,
) -> Result<(WicCreatePreview, PathBuf), String> {
    let kick = WicKickstart {
        identity: WicKickstartIdentity {
            name: w.kickstart_name,
            path: w.kickstart_path.map(Into::into),
        },
        source: String::new(),
        partitions: Vec::new(),
        limitations: Vec::new(),
    };
    let compression = match w.compression.as_str() {
        "None" => WicCompression::None,
        "Gzip" => WicCompression::Gzip,
        "Bzip2" => WicCompression::Bzip2,
        "Xz" => WicCompression::Xz,
        _ => return Err("invalid Wic compression".into()),
    };
    let output: PathBuf = w.output_directory.clone().into();
    let req = WicCreateRequest {
        machine: w.machine,
        image: w.image,
        kickstart: kick.identity.clone(),
        output_directory: output.clone(),
        generate_bmap: w.generate_bmap,
        compression,
    };
    req.validate().map_err(str::to_owned)?;
    let mut argv = vec![
        executable.into(),
        "create".into(),
        kick.identity.argument(),
        "-e".into(),
        req.image.clone().into(),
        "-o".into(),
        req.output_directory.clone(),
    ];
    if req.generate_bmap {
        argv.push("--bmap".into());
    }
    if let Some(c) = req.compression.argument() {
        argv.extend(["--compress-with".into(), c.into()]);
    }
    Ok((
        WicCreatePreview {
            request: req,
            kickstart: kick,
            argv,
        },
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_runtime_wic_create_reconstructs_typed_preview() {
        let request = DaemonWicCreateRequest {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart_name: "directdisk".into(),
            kickstart_path: None,
            output_directory: "/tmp/wic-output".into(),
            generate_bmap: true,
            compression: "Gzip".into(),
        };
        let (preview, output) = wire_preview(request, "/usr/bin/wic".into()).unwrap();
        assert_eq!(preview.request.image, "core-image-minimal");
        assert_eq!(output, PathBuf::from("/tmp/wic-output"));
        assert!(preview.argv.iter().any(|arg| arg == "--bmap"));
    }
}
