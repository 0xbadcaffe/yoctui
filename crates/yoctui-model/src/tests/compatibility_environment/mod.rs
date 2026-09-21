use super::*;

fn full_identity() -> YoctoEnvironmentIdentity {
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            PathBuf::from("/work/poky/build"),
            IdentityAuthority::InitializedEnvironment,
        ),
        source_roots: AuthoritativeValue::detected(
            vec![
                SourceRootIdentity {
                    kind: SourceRootKind::Layer,
                    path: "/work/poky/meta-poky".into(),
                },
                SourceRootIdentity {
                    kind: SourceRootKind::CoreBase,
                    path: "/work/poky".into(),
                },
            ],
            IdentityAuthority::ConfiguredLayerMetadata,
        ),
        bitbake_version: AuthoritativeValue::detected(
            "2.8.1".into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        oe_core: AuthoritativeValue::detected(
            ReleaseIdentity {
                name: Some("scarthgap".into()),
                version: Some("5.0.19".into()),
            },
            IdentityAuthority::ReleaseMetadata,
        ),
        poky: AuthoritativeValue::detected(
            ReleaseIdentity {
                name: Some("scarthgap".into()),
                version: Some("5.0.19".into()),
            },
            IdentityAuthority::ReleaseMetadata,
        ),
        distro: AuthoritativeValue::detected(
            DistroIdentity {
                name: "poky".into(),
                version: Some("5.0.19".into()),
            },
            IdentityAuthority::BitBakeDatastore,
        ),
        machine: AuthoritativeValue::detected(
            "qemux86-64".into(),
            IdentityAuthority::BitBakeDatastore,
        ),
        layer_series: AuthoritativeValue::detected(
            vec![
                LayerSeriesIdentity {
                    layer: "meta-poky".into(),
                    root: "/work/poky/meta-poky".into(),
                    compatible_series: vec!["scarthgap".into()],
                },
                LayerSeriesIdentity {
                    layer: "core".into(),
                    root: "/work/poky/meta".into(),
                    compatible_series: vec!["scarthgap".into(), "nanbield".into()],
                },
            ],
            IdentityAuthority::ConfiguredLayerMetadata,
        ),
        available_tools: AuthoritativeValue::detected(
            vec![ToolIdentity {
                id: "bitbake".into(),
                executable: "/work/poky/bitbake/bin/bitbake".into(),
                version: Some("2.8.1".into()),
            }],
            IdentityAuthority::ExecutableProbe,
        ),
        backend: AuthoritativeValue::detected(
            BackendIdentity {
                name: "tinfoil".into(),
                version: Some("1".into()),
            },
            IdentityAuthority::BackendHandshake,
        ),
        protocol: AuthoritativeValue::detected(
            ProtocolIdentity {
                name: "yoctui-daemon".into(),
                version: "1.0".into(),
            },
            IdentityAuthority::ProtocolNegotiation,
        ),
    }
}

mod environment_identity_normalizes_deterministically_without_collapsing_mixed_series;

mod environment_identity_preserves_unknown_for_every_partial_field;

mod environment_identity_rejects_weak_or_wrong_authority;

mod environment_identity_rejects_invalid_paths_text_and_empty_detected_collections;

mod environment_identity_rejects_conflicting_duplicate_tools_and_layers;

mod environment_identity_deduplicates_exact_authoritative_records;

mod compatibility_future_unknown_identity_is_preserved_without_release_inference;
