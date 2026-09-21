use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use serde::de::{Deserializer as _, IgnoredAny, MapAccess, Visitor};
use thiserror::Error;
use yoctui_model::{
    ImageArtifactIdentity, MAX_ROOTFS_DEPTH, MAX_ROOTFS_ENTRIES, MAX_ROOTFS_PACKAGES,
    MAX_ROOTFS_SYSTEM_PREVIEW_BYTES, PackageIdentity, RootfsAuthority, RootfsComposition,
    RootfsCompositionRequest, RootfsDbusService, RootfsEntry, RootfsEntryKind,
    RootfsFilesystemTree, RootfsInstalledPackage, RootfsPackageInventory, RootfsPathIdentity,
    RootfsSystemInventory, RootfsSystemdService,
};

const ROOTFS_SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PKGDATA_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ROOTFS_ACCOUNTED_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_LIMITATIONS: usize = 64;
const MAX_SYSTEM_RECORDS: usize = 4_096;
const MAX_SYSTEM_FILE_BYTES: u64 = 1024 * 1024;

mod udev;

include!("rootfs/types_and_adapter.rs");
include!("rootfs/source_and_system_scan.rs");
include!("rootfs/manifest_and_pkgdata.rs");
include!("rootfs/package_and_filesystem.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture() -> (PathBuf, RootfsCompositionRequest, RootfsCompositionSources) {
        let build = std::env::temp_dir().join(format!(
            "yoctui-rootfs-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let deploy = build.join("tmp/deploy/images/qemux86-64");
        let pkgdata = build.join("tmp/pkgdata/qemux86-64/runtime");
        let rootfs = build.join("tmp/work/qemux86-64/image/1.0-r0/rootfs");
        fs::create_dir_all(&deploy).unwrap();
        fs::create_dir_all(&pkgdata).unwrap();
        fs::create_dir_all(rootfs.join("usr/bin")).unwrap();
        fs::create_dir_all(rootfs.join("usr/lib/systemd/system")).unwrap();
        fs::create_dir_all(rootfs.join("etc/systemd/system/multi-user.target.wants")).unwrap();
        fs::create_dir_all(rootfs.join("usr/share/dbus-1/system-services")).unwrap();
        fs::create_dir_all(rootfs.join("usr/share/dbus-1/system.d")).unwrap();
        let artifact = deploy.join("core-image-minimal.rootfs.ext4");
        fs::write(&artifact, b"image").unwrap();
        let manifest = deploy.join("core-image-minimal.rootfs.manifest");
        fs::write(
            &manifest,
            "busybox qemux86_64 1.0\nbase-files qemux86_64 1.0\n",
        )
        .unwrap();
        fs::write(
            pkgdata.join("busybox"),
            "PN: busybox\nSECTION: base\nPKGSIZE:busybox: 12\nFILES_INFO:busybox: {\"/usr/bin/busybox\":{}}\n",
        )
        .unwrap();
        fs::write(
            pkgdata.join("base-files"),
            "PN: base-files\nSECTION: base\nPKGSIZE: 5\nFILES_INFO: {\"/etc/os-release\":{},\"/etc/passwd\":{}}\n",
        )
        .unwrap();
        fs::write(rootfs.join("usr/bin/busybox"), b"busybox").unwrap();
        fs::write(
            rootfs.join("usr/lib/systemd/system/example.service"),
            b"[Unit]\nDescription=Example daemon\n[Service]\nBusName=org.example.Daemon\nExecStart=/usr/bin/example\n",
        )
        .unwrap();
        fs::write(
            rootfs.join("usr/share/dbus-1/system-services/org.example.Helper.service"),
            b"[D-BUS Service]\nName=org.example.Helper\nExec=/usr/bin/helper\nUser=root\nSystemdService=example.service\n",
        )
        .unwrap();
        fs::write(
            rootfs.join("usr/share/dbus-1/system.d/example.conf"),
            b"<busconfig><policy><allow own=\"org.example.Helper\"/></policy></busconfig>\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("busybox", rootfs.join("usr/bin/sh")).unwrap();
            std::os::unix::fs::symlink(
                "../../../usr/lib/systemd/system/example.service",
                rootfs.join("etc/systemd/system/multi-user.target.wants/example.service"),
            )
            .unwrap();
        }
        let image = ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: artifact,
        };
        let request = RootfsCompositionRequest {
            generation: 4,
            image: image.clone(),
        };
        let sources = RootfsCompositionSources {
            image,
            manifest: Some(manifest),
            pkgdata_directory: Some(build.join("tmp/pkgdata/qemux86-64")),
            image_rootfs: Some(rootfs),
        };
        (build, request, sources)
    }

    #[tokio::test]
    async fn ux_rootfs_acquires_exact_manifest_pkgdata_and_no_follow_tree() {
        let (build, request, sources) = fixture();
        let expected_root = sources.image_rootfs.clone().unwrap();
        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request.clone())
            .await
            .unwrap();
        assert_eq!(response.request, request);
        let packages = response.composition.package_inventory().unwrap();
        assert_eq!(packages.packages.len(), 2);
        assert_eq!(packages.packages[0].identity.name, "base-files");
        assert_eq!(packages.packages[0].file_count, 2);
        assert_eq!(packages.packages[1].installed_size_bytes, 12);
        let entries = &response.composition.filesystem_tree().unwrap().entries;
        assert!(entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/usr/bin/busybox")
                && entry.kind == RootfsEntryKind::RegularFile
                && entry.size_bytes == 7
        }));
        #[cfg(unix)]
        assert!(entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/usr/bin/sh") && entry.kind == RootfsEntryKind::Symlink
        }));
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("ownership"))
        );
        let system = response.composition.system_inventory().unwrap();
        let unit = system
            .systemd_services
            .iter()
            .find(|service| service.name == "example.service")
            .unwrap();
        assert_eq!(unit.description.as_deref(), Some("Example daemon"));
        assert_eq!(unit.bus_name.as_deref(), Some("org.example.Daemon"));
        #[cfg(unix)]
        assert_eq!(unit.enabled_by, ["multi-user.target.wants"]);
        let activation = system
            .dbus_services
            .iter()
            .find(|service| service.name == "org.example.Helper")
            .unwrap();
        assert_eq!(
            activation.systemd_service.as_deref(),
            Some("example.service")
        );
        assert_eq!(activation.policy_files.len(), 1);
        assert!(
            system
                .dbus_services
                .iter()
                .any(|service| service.name == "org.example.Daemon")
        );
        assert_eq!(
            response.composition.root_directory.as_deref(),
            Some(expected_root.as_path())
        );
        fs::remove_dir_all(build).unwrap();
    }

    #[test]
    fn ux_rootfs_streams_large_scoped_wrynose_pkgdata_and_counts_files() {
        let (build, _, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.unwrap();
        let runtime = pkgdata.join("runtime");
        let files = (0..20_000)
            .map(|index| format!("\"/usr/src/kernel/file-{index}\":{{}}"))
            .collect::<Vec<_>>()
            .join(",");
        let content = format!(
            "PN: kernel-devsrc\nSECTION: kernel\nFILES_INFO:kernel-devsrc: {{{files}}}\nPKGSIZE:kernel-devsrc: 74306744\n"
        );
        assert!(content.len() > 256 * 1024);
        let path = runtime.join("kernel-devsrc");
        fs::write(&path, content).unwrap();

        let mut total = 0;
        let values = read_pkgdata(&path, &pkgdata, "kernel-devsrc", &mut total).unwrap();
        assert_eq!(values.recipe.as_deref(), Some("kernel-devsrc"));
        assert_eq!(values.category.as_deref(), Some("kernel"));
        assert_eq!(values.installed_size, Some(74_306_744));
        assert_eq!(values.file_count, Some(20_000));
        assert!(values.files_info_seen);
        assert_eq!(total, fs::metadata(path).unwrap().len());
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rootfs_runtime_reverse_preserves_installed_identity_and_scoped_fields() {
        let (build, request, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.as_ref().unwrap();
        fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
        fs::write(
            sources.manifest.as_ref().unwrap(),
            "libblkid1 arm1176jzs 1.0\nlibblkid1 arm1176jzs 1.0\nbusybox arm1176jzs 1.0\n",
        )
        .unwrap();
        fs::write(
            pkgdata.join("runtime/util-linux-libblkid"),
            "PN: util-linux\nPKG:util-linux-libblkid: libblkid1\nSECTION: base\nPKGSIZE:util-linux-libblkid: 354189\nFILES_INFO:util-linux-libblkid: {\"/usr/lib/libblkid.so.1\":17,\"/usr/lib/libblkid.so.1.1.0\":354172}\n",
        ).unwrap();
        std::os::unix::fs::symlink(
            "../runtime/util-linux-libblkid",
            pkgdata.join("runtime-reverse/libblkid1"),
        )
        .unwrap();
        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request)
            .await
            .unwrap();
        assert!(matches!(
            response.composition.installed_packages,
            RootfsAuthority::Available(_)
        ));
        let packages = &response.composition.package_inventory().unwrap().packages;
        assert_eq!(packages.len(), 2);
        let renamed = packages
            .iter()
            .find(|p| p.identity.name == "libblkid1")
            .unwrap();
        assert_eq!(renamed.recipe.as_deref(), Some("util-linux"));
        assert_eq!(renamed.installed_size_bytes, 354189);
        assert_eq!(renamed.file_count, 2);
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rootfs_runtime_reverse_rejects_unsafe_missing_and_conflicting_mappings() {
        use std::os::unix::fs::symlink;
        let (build, _, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.unwrap();
        let reverse = pkgdata.join("runtime-reverse");
        fs::create_dir(&reverse).unwrap();
        let runtime = pkgdata.join("runtime");
        fs::write(
            runtime.join("internal"),
            "PKG:internal: renamed\nPKGSIZE:internal: 8\nFILES_INFO:internal: {}\n",
        )
        .unwrap();
        for (name, target) in [
            ("absolute", runtime.join("internal")),
            ("escape", PathBuf::from("../../outside")),
            ("nested", PathBuf::from("../runtime/sub/internal")),
            ("dangling", PathBuf::from("../runtime/missing")),
            ("loop", PathBuf::from("loop")),
            ("wrong-pkg", PathBuf::from("../runtime/internal")),
        ] {
            symlink(target, reverse.join(name)).unwrap();
            let mut bytes = 0;
            assert!(
                read_installed_pkgdata(&pkgdata, name, &mut bytes).is_err(),
                "{name}"
            );
        }
        // A reverse entry cannot itself be an arbitrary regular metadata file.
        fs::write(reverse.join("regular"), "PKGSIZE: 99\n").unwrap();
        assert!(read_installed_pkgdata(&pkgdata, "regular", &mut 0).is_err());
        symlink("internal", runtime.join("chain")).unwrap();
        symlink("../runtime/chain", reverse.join("chain")).unwrap();
        assert!(read_installed_pkgdata(&pkgdata, "chain", &mut 0).is_err());
        // A conflicting direct record does not override the final-name index.
        fs::write(runtime.join("renamed"), "PKG: different\nPKGSIZE: 999\n").unwrap();
        symlink("../runtime/internal", reverse.join("renamed")).unwrap();
        assert_eq!(
            read_installed_pkgdata(&pkgdata, "renamed", &mut 0)
                .unwrap()
                .installed_size,
            Some(8)
        );
        // Mapped aliases cannot count the same record under a second identity.
        symlink("../runtime/internal", reverse.join("alias")).unwrap();
        assert!(read_installed_pkgdata(&pkgdata, "alias", &mut 0).is_err());
        fs::write(
            runtime.join("internal"),
            "PKG:internal: renamed\nPKG:internal: conflict\nPKG:internal: renamed\n",
        )
        .unwrap();
        assert!(read_installed_pkgdata(&pkgdata, "renamed", &mut 0).is_err());
        fs::write(runtime.join("internal"), "PKGSIZE:internal: 8\n").unwrap();
        assert!(read_installed_pkgdata(&pkgdata, "renamed", &mut 0).is_err());
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rootfs_runtime_reverse_preserves_file_and_total_byte_limits() {
        let (build, _, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.unwrap();
        fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
        std::os::unix::fs::symlink(
            "../runtime/busybox",
            pkgdata.join("runtime-reverse/busybox"),
        )
        .unwrap();
        let mut total_bytes = MAX_PKGDATA_TOTAL_BYTES;
        assert!(matches!(
            read_installed_pkgdata(&pkgdata, "busybox", &mut total_bytes),
            Err(RootfsCompositionAdapterError::ResourceLimit(_))
        ));
        fs::File::create(pkgdata.join("runtime/busybox"))
            .unwrap()
            .set_len(MAX_PKGDATA_FILE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            read_installed_pkgdata(&pkgdata, "busybox", &mut 0),
            Err(RootfsCompositionAdapterError::ResourceLimit(_))
        ));
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rootfs_runtime_reverse_partial_scan_retains_control_and_containment() {
        let (build, request, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.as_ref().unwrap();
        fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
        std::os::unix::fs::symlink(
            "../runtime/missing",
            pkgdata.join("runtime-reverse/busybox"),
        )
        .unwrap();
        let response = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4)
            .scan(request.clone())
            .await
            .unwrap();
        assert!(matches!(
            response.composition.installed_packages,
            RootfsAuthority::Partial { .. }
        ));
        let packages = &response.composition.package_inventory().unwrap().packages;
        assert_eq!(packages.len(), 2);
        assert_eq!(
            packages
                .iter()
                .find(|p| p.identity.name == "base-files")
                .unwrap()
                .installed_size_bytes,
            5
        );
        assert_eq!(
            packages
                .iter()
                .find(|p| p.identity.name == "busybox")
                .unwrap()
                .installed_size_bytes,
            0
        );
        let cancellation = RootfsCompositionCancellation::default();
        cancellation.cancel();
        assert_eq!(
            RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4)
                .scan_with_cancellation(request.clone(), cancellation)
                .await,
            Err(RootfsCompositionAdapterError::Cancelled)
        );
        assert!(matches!(
            scan_sources(
                request,
                build.clone(),
                sources.clone(),
                RootfsCompositionCancellation::default(),
                Instant::now()
            ),
            Err(RootfsCompositionAdapterError::Timeout(_))
        ));
        fs::rename(
            pkgdata.join("runtime-reverse"),
            pkgdata.join("saved-reverse"),
        )
        .unwrap();
        std::os::unix::fs::symlink("saved-reverse", pkgdata.join("runtime-reverse")).unwrap();
        assert!(read_installed_pkgdata(pkgdata, "busybox", &mut 0).is_err());
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn ux_rootfs_oversized_single_pkgdata_is_partial_not_a_screen_failure() {
        let (build, request, mut sources) = fixture();
        let manifest = sources.manifest.as_ref().unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(manifest)
            .unwrap()
            .write_all(b"oversized qemux86_64 1.0\n")
            .unwrap();
        let oversized = sources
            .pkgdata_directory
            .as_ref()
            .unwrap()
            .join("runtime/oversized");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_PKGDATA_FILE_BYTES + 1)
            .unwrap();
        sources.image_rootfs = None;

        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request)
            .await
            .unwrap();
        assert!(matches!(
            response.composition.installed_packages,
            RootfsAuthority::Partial { .. }
        ));
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("oversized") && value.contains("limited"))
        );
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ux_rootfs_deduplicates_hardlink_bytes_and_accounts_special_files() {
        use std::os::unix::net::UnixListener;

        let (build, request, sources) = fixture();
        let root = sources.image_rootfs.as_ref().unwrap();
        fs::hard_link(
            root.join("usr/bin/busybox"),
            root.join("usr/bin/busybox.link"),
        )
        .unwrap();
        let socket = root.join("run.sock");
        let _listener = UnixListener::bind(&socket).unwrap();
        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request)
            .await
            .unwrap();
        let tree = response.composition.filesystem_tree().unwrap();
        let hardlink_bytes = tree
            .entries
            .iter()
            .filter(|entry| entry.identity.0.to_string_lossy().contains("busybox"))
            .map(|entry| entry.size_bytes)
            .sum::<u64>();
        assert_eq!(hardlink_bytes, 7);
        assert!(tree.entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/run.sock") && entry.kind == RootfsEntryKind::Other
        }));
        drop(_listener);
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn ux_rootfs_denies_stale_cancelled_mismatched_and_escaping_sources() {
        let (build, request, sources) = fixture();
        let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 5);
        assert!(matches!(
            adapter.scan(request.clone()).await,
            Err(RootfsCompositionAdapterError::StaleGeneration { .. })
        ));
        let cancellation = RootfsCompositionCancellation::default();
        cancellation.cancel();
        let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4);
        assert_eq!(
            adapter
                .scan_with_cancellation(request.clone(), cancellation)
                .await,
            Err(RootfsCompositionAdapterError::Cancelled)
        );
        let mut mismatch = request.clone();
        mismatch.image.image = "another-image".into();
        assert_eq!(
            adapter.scan(mismatch).await,
            Err(RootfsCompositionAdapterError::ImageMismatch)
        );
        let outside = std::env::temp_dir().join(format!("yoctui-outside-{}", std::process::id()));
        fs::create_dir_all(&outside).unwrap();
        let mut escaping = sources;
        escaping.image_rootfs = Some(outside.clone());
        let adapter = RootfsCompositionAdapter::new(build.clone(), escaping, 4);
        assert!(matches!(
            adapter.scan(request).await,
            Err(RootfsCompositionAdapterError::PathEscape(_))
        ));
        fs::remove_dir_all(build).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
