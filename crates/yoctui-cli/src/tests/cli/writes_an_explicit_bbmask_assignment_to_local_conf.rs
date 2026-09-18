use super::*;

#[tokio::test]
async fn writes_an_explicit_bbmask_assignment_to_local_conf() {
    let build_dir = std::env::temp_dir().join(format!("yoctui-bbmask-{}", std::process::id()));
    let conf_dir = build_dir.join("conf");
    fs::create_dir_all(&conf_dir).unwrap();
    let local_conf = conf_dir.join("local.conf");
    fs::write(&local_conf, "MACHINE = \"qemuarm\"\n").unwrap();

    write_bbmask(&build_dir, "meta-broken/.*".into())
        .await
        .unwrap();

    assert_eq!(
        fs::read_to_string(&local_conf).unwrap(),
        "MACHINE = \"qemuarm\"\nBBMASK = \"meta-broken/.*\"\n"
    );
    fs::remove_file(local_conf).unwrap();
    fs::remove_dir(conf_dir).unwrap();
    fs::remove_dir(build_dir).unwrap();
}
