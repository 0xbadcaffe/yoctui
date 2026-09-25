use super::*;

#[test]
fn client_runtime_devtool_maps_every_effect_to_closed_wire_type() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/busybox.bb".into(),
    };
    assert_eq!(
        daemon_command_for_effect(&app, &Effect::InspectDevtoolStatus(identity.clone())).unwrap(),
        Some(DaemonCommand::InspectDevtoolStatus {
            recipe: "busybox".into(),
            recipe_file: "/layers/busybox.bb".into(),
            build_directory: "/build".into(),
        })
    );
    let cases = [
        (
            Effect::DevtoolModify(identity.clone()),
            DaemonDevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
        ),
        (
            Effect::DevtoolUpdateRecipe(identity.clone()),
            DaemonDevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
        ),
        (
            Effect::DevtoolReset(yoctui_model::DevtoolResetPlan {
                identity: identity.clone(),
                source_path: "/workspace/busybox".into(),
            }),
            DaemonDevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
        ),
        (
            Effect::DevtoolFinish(yoctui_model::DevtoolFinishPlan {
                identity: identity.clone(),
                layer: yoctui_model::Layer {
                    name: "meta-test".into(),
                    path: "/layers/meta-test".into(),
                    priority: Some(7),
                },
            }),
            DaemonDevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta-test".into(),
            },
        ),
        (
            Effect::DevtoolDeploy(yoctui_model::DevtoolDeployPlan {
                identity: identity.clone(),
                target: "root@example".into(),
            }),
            DaemonDevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@example".into(),
            },
        ),
        (
            Effect::DevtoolUndeploy(yoctui_model::DevtoolUndeployPlan {
                identity: identity.clone(),
                target: "root@example".into(),
            }),
            DaemonDevtoolOperation::UndeployTarget {
                recipe: "busybox".into(),
                target: "root@example".into(),
            },
        ),
        (
            Effect::DevtoolUpgrade(yoctui_model::DevtoolUpgradePlan { identity }),
            DaemonDevtoolOperation::Upgrade {
                recipe: "busybox".into(),
            },
        ),
    ];
    for (effect, expected) in cases {
        assert_eq!(
            daemon_command_for_effect(&app, &effect).unwrap(),
            Some(DaemonCommand::StartDevtool {
                operation: expected,
                build_directory: "/build".into(),
            })
        );
    }
    assert_eq!(
        daemon_command_for_effect(&app, &Effect::PersistSettings).unwrap(),
        None
    );
}
