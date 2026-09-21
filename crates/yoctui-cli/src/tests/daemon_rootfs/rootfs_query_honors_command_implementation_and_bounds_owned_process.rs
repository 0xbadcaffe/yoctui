use super::*;

#[tokio::test]
async fn rootfs_query_honors_command_implementation_and_bounds_owned_process() {
    for implementation in ["bitbake_getvar.argv", "bitbake.environment_lookup"] {
        for mode in ["ok", "absent", "error", "hang", "cancel", "oversized"] {
            let (build, query, mut compatibility, mut environment) = fixture();
            let executable = build.join("getvar-fixture");
            fs::write(&executable, r#"#!/usr/bin/python3
import os, sys, time
from pathlib import Path
Path('command.pid').write_text(str(os.getpid()))
assert os.environ['ROOTFS_MARKER']=='daemon-environment'
mode=os.environ['ROOTFS_TEST_MODE']
assert sys.argv[1:]==['-e','image'] or (sys.argv[1:4]==['--value','--recipe','image'] and len(sys.argv)==5)
if mode in ('hang','cancel'): time.sleep(60)
if mode=='error': sys.exit('fixture command error')
if mode=='oversized':
    sys.stdout.write('x' * (17 * 1024 * 1024)); sys.stdout.flush(); time.sleep(60)
values={k:str(Path.cwd()/v) for k,v in [('IMAGE_MANIFEST','image.manifest'),('PKGDATA_DIR','pkgdata'),('IMAGE_ROOTFS','retained-rootfs')]}
if sys.argv[1]=='-e':
    for name,value in values.items(): print(name+'="'+('' if mode=='absent' else value)+'"')
else: print('' if mode=='absent' else values[sys.argv[-1]])
"#).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            let selected = compatibility
                .implementations
                .get_mut(&CapabilityId::BitBakeGetVar)
                .unwrap();
            selected.id = implementation.into();
            selected.kind = CapabilityImplementationKind::Command;
            if let AuthoritativeValue::Detected { value: tools, .. } =
                &mut compatibility.snapshot.environment.available_tools
            {
                for tool in tools {
                    if matches!(tool.id.as_str(), "bitbake-getvar" | "bitbake") {
                        tool.executable = executable.clone();
                    }
                }
            } else {
                panic!("fixture tools must be detected");
            }
            environment.insert("ROOTFS_MARKER".into(), "daemon-environment".into());
            environment.insert("ROOTFS_TEST_MODE".into(), mode.into());
            let (cancel, cancelled) = oneshot::channel();
            let cancel_build = build.clone();
            let cancellation = tokio::spawn(async move {
                if mode == "cancel" {
                    for _ in 0..100 {
                        if cancel_build.join("command.pid").exists() {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    let _ = cancel.send(());
                } else {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    drop(cancel);
                }
            });
            let result = acquire(
                query.clone(),
                build.clone(),
                compatibility,
                environment,
                cancelled,
                if mode == "hang" {
                    Duration::from_millis(300)
                } else {
                    Duration::from_secs(5)
                },
            )
            .await;
            cancellation.abort();
            if matches!(mode, "ok" | "absent") {
                let sources = result.unwrap();
                assert_eq!(sources.query, query);
                assert_eq!(
                    sources.image_rootfs,
                    (mode == "ok").then(|| build.join("retained-rootfs").display().to_string())
                );
            } else {
                let error = format!("{:#}", result.unwrap_err());
                assert!(
                    error.contains(match mode {
                        "error" => "fixture command error",
                        "hang" => "timed out",
                        "cancel" => "cancelled",
                        _ => "output bound",
                    }),
                    "{error}"
                );
            }
            let pid: i32 = fs::read_to_string(build.join("command.pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
                -1
            );
            assert!(!Path::new(&format!("/proc/{pid}")).exists());
            fs::remove_dir_all(build).unwrap();
        }
    }
}
