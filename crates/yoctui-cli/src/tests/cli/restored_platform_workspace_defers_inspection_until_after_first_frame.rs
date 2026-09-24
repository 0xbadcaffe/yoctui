use super::*;

#[test]
fn restored_platform_workspace_defers_inspection_until_after_first_frame() {
    for screen in [Screen::Kernel, Screen::Firmware] {
        let mut app = App::new(8, 1_000);
        app.screen = screen;
        app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
        app.build_environment = yoctui_model::BuildEnvironmentState::Configured(
            yoctui_model::BuildEnvironmentProfile {
                init_script: "/source/oe-init-build-env".into(),
                source_dir: "/source".into(),
                build_dir: "/build".into(),
            },
        );
        let capabilities = [
            yoctui_model::CapabilityId::BitBakeGetVar,
            yoctui_model::CapabilityId::BitBakeRecipeMetadata,
        ];
        let authority = yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: yoctui_model::CapabilitySnapshot {
                generation: 1,
                environment: yoctui_model::YoctoEnvironmentIdentity::default(),
                capabilities: capabilities
                    .into_iter()
                    .map(|id| yoctui_model::CapabilityRecord {
                        id,
                        state: yoctui_model::CapabilityState::Available,
                        evidence: vec![yoctui_model::CapabilityEvidence {
                            kind: yoctui_model::CapabilityEvidenceKind::BackendNegotiation,
                            outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                            subject: id.as_str().into(),
                            detail: "test bridge capability".into(),
                            argv: vec![],
                        }],
                    })
                    .collect(),
            },
            implementations: std::collections::BTreeMap::from([
                (
                    yoctui_model::CapabilityId::BitBakeGetVar,
                    yoctui_model::CapabilityImplementation {
                        id: "bitbake_getvar.argv".into(),
                        kind: yoctui_model::CapabilityImplementationKind::Command,
                    },
                ),
                (
                    yoctui_model::CapabilityId::BitBakeRecipeMetadata,
                    yoctui_model::CapabilityImplementation {
                        id: "tinfoil.recipe_metadata".into(),
                        kind: yoctui_model::CapabilityImplementationKind::BackendApi,
                    },
                ),
            ]),
        }
        .normalize()
        .unwrap();
        yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();

        assert_eq!(begin_startup_platform_inspection(&mut app), Some(screen));
        let inventory = if screen == Screen::Kernel {
            &app.kernel.inventory
        } else {
            &app.firmware.inventory
        };
        assert!(matches!(
            inventory,
            yoctui_model::PlatformInventoryState::Loading
        ));
    }
}
