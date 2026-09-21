use super::*;

fn image() -> ImageArtifactIdentity {
    ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/core-image-minimal-qemux86-64.rootfs.ext4".into(),
    }
}

mod image_console_qemu_enforces_serial_stdio_and_nographic;

mod image_console_ssh_is_argv_only_and_keeps_host_key_defaults;

mod image_console_rejects_ambiguous_ssh_inputs_and_bounds_fields;
