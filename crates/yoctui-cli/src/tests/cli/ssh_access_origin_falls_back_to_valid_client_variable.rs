use super::*;

#[test]
fn ssh_access_origin_falls_back_to_valid_client_variable() {
    assert_eq!(
        client_access_origin_from_values(["", "198.51.100.27 49152 198.51.100.10 22",]),
        ClientAccessOrigin::Ssh {
            client_ip: "198.51.100.27".into(),
        }
    );
    assert_eq!(
        client_access_origin_from_values(["invalid", "also-invalid"]),
        ClientAccessOrigin::SshUnknown
    );
    assert_eq!(
        client_access_origin_from_values(std::iter::empty()),
        ClientAccessOrigin::Local
    );
}
