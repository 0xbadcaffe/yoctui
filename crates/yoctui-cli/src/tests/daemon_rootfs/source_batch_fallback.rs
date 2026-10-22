use super::*;

#[tokio::test]
async fn rootfs_batch_uses_only_authorized_dump_and_preserves_single_name_utility_fallback() {
    for implementation in ["bitbake_getvar.argv", "bitbake.environment_lookup"] {
        for authorized_dump in [true, false] {
            let (build, query, mut compatibility, environment) = fixture();
            let executable = build.join("getvar-fixture");
            fs::write(
                &executable,
                r#"#!/usr/bin/python3
import sys
from pathlib import Path
with Path('commands').open('a') as log: log.write(' '.join(sys.argv[1:])+'\n')
values={'IMAGE_MANIFEST':'image.manifest','PKGDATA_DIR':'pkgdata','IMAGE_ROOTFS':'rootfs'}
if sys.argv[1:]==['-e','image']:
    print('IMAGE_ROOTFS="/stale"')
    for name,value in values.items(): print('export '+name+'="'+str(Path.cwd()/value)+'"')
else:
    assert sys.argv[1:4]==['--value','--recipe','image'] and len(sys.argv)==5
    print(Path.cwd()/values[sys.argv[4]])
"#,
            )
            .unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            let selected = compatibility
                .implementations
                .get_mut(&CapabilityId::BitBakeGetVar)
                .unwrap();
            selected.id = implementation.into();
            selected.kind = CapabilityImplementationKind::Command;
            if !authorized_dump {
                compatibility
                    .implementations
                    .remove(&CapabilityId::BitBakeEnvironmentDump);
            }
            if let AuthoritativeValue::Detected { value: tools, .. } =
                &mut compatibility.snapshot.environment.available_tools
            {
                for tool in tools {
                    if matches!(tool.id.as_str(), "bitbake-getvar" | "bitbake") {
                        tool.executable = executable.clone();
                    }
                }
            }
            let (_cancel, cancelled) = oneshot::channel();
            let result = acquire(
                query,
                build.clone(),
                compatibility,
                environment,
                cancelled,
                Duration::from_secs(3),
            )
            .await
            .unwrap();
            assert_eq!(
                result.image_manifest,
                Some(build.join("image.manifest").display().to_string())
            );
            assert_eq!(
                result.pkgdata_dir,
                Some(build.join("pkgdata").display().to_string())
            );
            assert_eq!(
                result.image_rootfs,
                Some(build.join("rootfs").display().to_string())
            );
            let commands = fs::read_to_string(build.join("commands")).unwrap();
            if authorized_dump || implementation == "bitbake.environment_lookup" {
                assert_eq!(commands, "-e image\n");
            } else {
                assert_eq!(
                    commands,
                    "--value --recipe image IMAGE_MANIFEST\n--value --recipe image PKGDATA_DIR\n--value --recipe image IMAGE_ROOTFS\n"
                );
            }
            fs::remove_dir_all(build).unwrap();
        }
    }
}
