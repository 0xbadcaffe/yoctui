use super::*;

#[tokio::test]
async fn compatibility_pkgdata_builds_exact_authorized_commands_and_parses_results() {
    let script = r#"#!/bin/sh
log="$(dirname "$0")/arguments.log"
printf '%s\n' "--" "$@" >> "$log"
case "$3" in
  list-pkgs)
    printf 'libc6\nbusybox\ninit\n'
    ;;
  package-info)
    printf 'busybox 1.37.0-r0 busybox 1.37.0-r0 1024 "GPL-2.0-only"\n'
    printf 'init 1.0-r0 init 1.0-r0 64 "MIT"\n'
    printf 'libc6 2.40-r0 glibc 2.40-r0 4096 "GPL-2.0-or-later"\n'
    ;;
  list-pkg-files)
    printf 'busybox:\n\t/bin/busybox\n\t/etc/busybox.conf\n'
    ;;
  read-value)
    printf 'busybox libc6 (>= 2.40)\n'
    printf 'init busybox\n'
    printf 'libc6\n'
    ;;
  *)
    printf 'unexpected subcommand\n' >&2
    exit 9
    ;;
esac
"#;
    let (_directory, adapter, log) = fixture("typed", script);
    let inventory = adapter
        .clone()
        .with_argument_batch(8)
        .inventory(inventory_request())
        .await
        .unwrap();
    assert_eq!(
        inventory
            .packages
            .iter()
            .map(|package| package.identity.name.as_str())
            .collect::<Vec<_>>(),
        vec!["busybox", "init", "libc6"]
    );
    let busybox = &inventory.packages[0];
    assert_eq!(busybox.recipe, PackageField::Available("busybox".into()));
    assert_eq!(busybox.version, PackageField::Available("1.37.0-r0".into()));
    assert_eq!(busybox.installed_size_bytes, PackageField::Available(1_024));
    assert_eq!(
        busybox.license,
        PackageField::Available("GPL-2.0-only".into())
    );
    assert_eq!(busybox.provider, PackageField::Unavailable);
    assert!(
        inventory
            .limitations
            .iter()
            .any(|limitation| limitation.contains("provider recipe paths"))
    );

    let detail = adapter
        .with_argument_batch(2)
        .detail(detail_request())
        .await
        .unwrap();
    assert_eq!(
        detail.detail.files,
        PackageField::Available(vec![
            PathBuf::from("/bin/busybox"),
            PathBuf::from("/etc/busybox.conf"),
        ])
    );
    assert_eq!(
        detail.detail.runtime_dependencies,
        PackageField::Available(vec![PackageIdentity::new("libc6")])
    );
    assert_eq!(
        detail.detail.reverse_dependencies,
        PackageField::Available(vec![PackageIdentity::new("init")])
    );

    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.contains("\n-p\n"));
    assert!(arguments.contains("\nlist-pkgs\n-r\n"));
    assert!(arguments.contains("\npackage-info\n-e\nLICENSE\nbusybox\n"));
    assert!(arguments.contains("\nlist-pkg-files\n-r\nbusybox\n"));
    assert!(arguments.contains("\nread-value\nRDEPENDS\n-n\n"));
}
