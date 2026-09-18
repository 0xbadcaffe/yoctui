use super::*;

#[test]
fn telemetry_sampling_derives_only_monotonic_identity_stable_rates() {
    let diskstats = "   8       0 sda 10 0 20 0 30 0 40 0 0 0 0 0 0 0 0\n";
    assert_eq!(
        parse_diskstats(diskstats, 8, 0),
        Some(DiskCounters {
            major: 8,
            minor: 0,
            read_bytes: 20 * 512,
            write_bytes: 40 * 512,
        })
    );
    assert_eq!(parse_diskstats(diskstats, 8, 1), None);

    let routes = concat!(
        "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n",
        "wlan0 00000000 01010101 0003 0 0 600 00000000 0 0 0\n",
        "eth0 00000000 01010101 0003 0 0 100 00000000 0 0 0\n",
        "down0 00000000 01010101 0000 0 0 1 00000000 0 0 0\n",
    );
    assert_eq!(
        parse_default_route_interface(routes).as_deref(),
        Some("eth0")
    );
    let network = parse_network_dev(
        "Inter-| Receive | Transmit\n eth0: 100 1 2 3 4 5 6 7 900 9 10 11 12 13 14 15\n",
        "eth0",
    )
    .unwrap();
    assert_eq!(network.receive_bytes, 100);
    assert_eq!(network.transmit_bytes, 900);

    assert_eq!(
        bytes_per_second(1_000, 3_000, Duration::from_millis(500)),
        Some(4_000)
    );
    assert_eq!(bytes_per_second(3_000, 1_000, Duration::from_secs(1)), None);
    assert_eq!(bytes_per_second(1_000, 3_000, Duration::ZERO), None);

    let previous_disk = DiskCounters {
        major: 8,
        minor: 0,
        read_bytes: 1_000,
        write_bytes: 2_000,
    };
    let current_disk = DiskCounters {
        major: 8,
        minor: 0,
        read_bytes: 3_000,
        write_bytes: 5_000,
    };
    assert_eq!(
        disk_rates(&previous_disk, &current_disk, Duration::from_secs(1)),
        Some((2_000, 3_000))
    );
    assert_eq!(
        disk_rates(
            &previous_disk,
            &DiskCounters {
                major: 8,
                minor: 1,
                ..current_disk
            },
            Duration::from_secs(1)
        ),
        None
    );

    let previous_network = NetworkCounters {
        interface: "eth0".into(),
        receive_bytes: 100,
        transmit_bytes: 200,
    };
    let current_network = NetworkCounters {
        interface: "eth0".into(),
        receive_bytes: 500,
        transmit_bytes: 800,
    };
    assert_eq!(
        network_rates(&previous_network, &current_network, Duration::from_secs(2)),
        Some((200, 300))
    );
    assert_eq!(
        network_rates(
            &previous_network,
            &NetworkCounters {
                interface: "wlan0".into(),
                ..current_network
            },
            Duration::from_secs(1)
        ),
        None
    );
}
