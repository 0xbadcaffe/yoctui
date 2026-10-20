use super::*;

#[test]
fn private_ssh_preparation_rejects_busy_port_before_starting_qemu() {
    let (_root, mut spec) = fixture();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    spec.ssh_forward_port = Some(port);
    let error = validate_files(&spec).unwrap_err();
    assert!(format!("{error:#}").contains("SSH port"));
    drop(listener);
    validate_files(&spec).unwrap();
    // Validation only: no guest or persistent listener is started.
    let _available = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
}

#[test]
fn private_ssh_does_not_guess_a_nic_for_an_unsupported_machine() {
    let (_root, mut spec) = fixture();
    spec.ssh_forward_port = Some(2245);
    fs::write(
        &spec.qemuboot,
        "[config_bsp]\nqb_system_name=qemu-system-aarch64\nqb_default_kernel=Image\n",
    )
    .unwrap();
    let error = validate_files(&spec).unwrap_err();
    assert!(error.to_string().contains("direct x86"));
    spec.ssh_forward_port = None;
    validate_files(&spec).unwrap();
}

#[test]
fn private_ssh_rejects_metadata_that_adds_network_or_listener_overrides() {
    let (_root, mut spec) = fixture();
    spec.ssh_forward_port = Some(2245);
    for extra in [
        "qb_opt_append=-netdev user,id=public,hostfwd=tcp::4444-:22",
        "qb_slirp_opt=-netdev user,id=net0",
        "qb_opt_append=-gdb tcp::1234",
        "qb_opt_append=-s",
        "qb_opt_append=-qmp tcp:0.0.0.0:4444,server=on,wait=off",
        "qb_opt_append=-readconfig /tmp/unreviewed.conf",
        "qb_opt_append=-vnc 0.0.0.0:0",
        "qb_opt_append=-spice port=5900,disable-ticketing=on",
        "qb_serial_opt=-serial tcp::4444,server=on,wait=off",
        "qb_opt_append=-incoming udp:0.0.0.0:4444",
    ] {
        fs::write(&spec.qemuboot, format!("[config_bsp]\nqb_system_name=qemu-system-x86_64\nqb_default_kernel=bzImage\n{extra}\n")).unwrap();
        let error = validate_files(&spec).unwrap_err();
        assert!(
            error.to_string().contains("network/listener overrides"),
            "{error:#}"
        );
    }
}
