use super::*;

#[test]
fn ssh_access_origin_uses_the_valid_remote_client_ip() {
    assert_eq!(
        parse_ssh_access_origin("192.0.2.44 50123 192.0.2.10 22"),
        ClientAccessOrigin::Ssh {
            client_ip: "192.0.2.44".into(),
        }
    );
    assert_eq!(
        parse_ssh_access_origin("2001:db8::44 50123 2001:db8::10 22"),
        ClientAccessOrigin::Ssh {
            client_ip: "2001:db8::44".into(),
        }
    );
    assert_eq!(
        parse_ssh_access_origin("not-an-ip 50123 local 22"),
        ClientAccessOrigin::SshUnknown
    );
}
