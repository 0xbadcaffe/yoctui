use super::*;

const SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (160, 50), (200, 60)];

#[test]
fn build_options_retains_every_action_and_escape_with_unknown_authority() {
    let mut app = App::new(32, 8192);
    app.build.target = Some("obmc-phosphor-image".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "romulus".into());
    app.dialogs.push_back(Dialog::BuildOptions);
    app.focus = FocusTarget::Dialog;
    for (width, height) in SIZES {
        let output = rendered_text_at(&app, width, height, literal_now());
        for anchor in [
            "Image build options",
            "Machine: romulus",
            "obmc-phosphor-image",
            "b  Build image",
            "c  Clean image",
            "m  Run menuconfig",
            "i  Choose image recipe",
            "e  Enter a target name",
            "Esc closes this menu.",
            "State: Unknown",
            "Confirmation disabled",
        ] {
            assert!(
                output.contains(anchor),
                "{width}x{height} lost {anchor}: {output}"
            );
        }
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    }
}

#[test]
fn deployment_draft_and_confirmation_retain_exact_identity_and_cancel_controls() {
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    for confirmation in [false, true] {
        let mut app = App::new(32, 8192);
        let dialog = if confirmation {
            Dialog::DevtoolDeployConfirmation(yoctui_model::DevtoolDeployPlan {
                identity: identity.clone(),
                target: "root@romulus".into(),
            })
        } else {
            Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                identity: identity.clone(),
                target: "root@romulus".into(),
            })
        };
        app.dialogs.push_back(dialog);
        app.focus = FocusTarget::Dialog;
        for (width, height) in SIZES {
            let output = rendered_text_at(&app, width, height, literal_now());
            for anchor in [
                "busybox.bb",
                "root@romulus",
                "Esc cancels.",
                "Confirmation disabled",
            ] {
                assert!(
                    output.contains(anchor),
                    "{width}x{height} lost {anchor}: {output}"
                );
            }
            let command = if confirmation {
                "devtool deploy-target busybox root@romulus"
            } else {
                "Enter previews the command"
            };
            assert!(output.contains(command), "{output}");
        }
    }
}

#[test]
fn deployment_long_values_never_displace_confirm_and_cancel_hints() {
    let mut app = App::new(32, 8192);
    app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
        yoctui_model::DevtoolDeployPlan {
            identity: yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: format!("/layers/{}/busybox.bb", "nested/".repeat(50)).into(),
            },
            target: format!("root@{}", "board".repeat(50)),
        },
    ));
    for (width, height) in SIZES {
        let output = rendered_text_at(&app, width, height, literal_now());
        assert!(output.contains("Enter continues; Esc cancels."), "{output}");
        assert!(output.contains("Confirmation disabled"), "{output}");
        assert!(!output.contains('�'), "{output}");
    }
}
