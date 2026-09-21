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

mod ux_rootfs_acquires_exact_manifest_pkgdata_and_no_follow_tree;

mod ux_rootfs_streams_large_scoped_wrynose_pkgdata_and_counts_files;

#[cfg(unix)]
mod rootfs_runtime_reverse_preserves_installed_identity_and_scoped_fields;

#[cfg(unix)]
mod rootfs_runtime_reverse_rejects_unsafe_missing_and_conflicting_mappings;

#[cfg(unix)]
mod rootfs_runtime_reverse_preserves_file_and_total_byte_limits;

#[cfg(unix)]
mod rootfs_runtime_reverse_partial_scan_retains_control_and_containment;

mod ux_rootfs_oversized_single_pkgdata_is_partial_not_a_screen_failure;

#[cfg(unix)]
mod ux_rootfs_deduplicates_hardlink_bytes_and_accounts_special_files;

mod ux_rootfs_denies_stale_cancelled_mismatched_and_escaping_sources;
