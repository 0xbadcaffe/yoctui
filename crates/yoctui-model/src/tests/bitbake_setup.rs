use super::*;

fn send(app: &mut App, action: BitBakeSetupAction) -> Option<Effect> {
    update(app, Action::BitBakeSetup(action))
}

fn values() -> [String; 5] {
    [
        "bitbake-setup",
        "/tmp/setup space",
        "demo",
        "poky-master",
        "",
    ]
    .map(str::to_owned)
}

#[test]
fn bitbake_setup_plan_is_argument_based_and_validates_paths() {
    let plan = BitBakeSetupPlan::from_values(&values()).unwrap();
    let request = plan.launch().unwrap();
    assert_eq!(request.program, PathBuf::from("bitbake-setup"));
    assert_eq!(plan.destination(), PathBuf::from("/tmp/setup space/demo"));
    assert!(request.arguments.contains(&"setup space".into()));
    assert!(!request.arguments.contains(&"--non-interactive".into()));
    for (field, text) in [
        (1, "/"),
        (1, "relative"),
        (1, "/tmp/../etc"),
        (2, "../bad"),
        (2, "-bad"),
        (3, "--help"),
        (4, "bad\nregistry"),
        (0, "sh"),
    ] {
        let mut invalid = values();
        invalid[field] = text.into();
        assert!(
            BitBakeSetupPlan::from_values(&invalid).is_err(),
            "{field}: {text}"
        );
    }
    let mut invalid = plan;
    invalid.top_dir = PathBuf::from("/");
    assert!(invalid.launch().is_err());
}

#[test]
fn bitbake_setup_requires_review_connection_and_ignores_cancelled_preparation() {
    let mut app = App::new(100, 65536);
    app.bitbake_setup.tool = "bitbake-setup".into();
    app.bitbake_setup.top_dir = "/tmp/setups".into();
    assert!(send(&mut app, BitBakeSetupAction::New).is_none());
    assert!(send(&mut app, BitBakeSetupAction::Confirm).is_none());
    assert!(send(&mut app, BitBakeSetupAction::Review).is_none());
    assert!(send(&mut app, BitBakeSetupAction::Confirm).is_none());
    app.daemon.status = ClientReplicaStatus::Current;
    let Some(Effect::BitBakeSetup(BitBakeSetupRequest::Prepare { generation, plan })) =
        send(&mut app, BitBakeSetupAction::Confirm)
    else {
        panic!("missing preparation")
    };
    send(&mut app, BitBakeSetupAction::Cancel);
    assert!(
        send(
            &mut app,
            BitBakeSetupAction::Prepared {
                generation,
                result: plan.launch()
            }
        )
        .is_none()
    );
    assert!(app.active_dialog().is_none());
}

#[test]
fn bitbake_setup_catalogue_rejects_stale_results_and_bounds_selection() {
    let mut app = App::new(100, 65536);
    send(&mut app, BitBakeSetupAction::Catalogue);
    let generation = app.bitbake_setup.generation;
    send(
        &mut app,
        BitBakeSetupAction::Listed {
            generation: generation - 1,
            result: Err("stale".into()),
        },
    );
    assert!(app.bitbake_setup.loading);
    send(
        &mut app,
        BitBakeSetupAction::Listed {
            generation,
            result: Ok(Vec::new()),
        },
    );
    send(&mut app, BitBakeSetupAction::Select(isize::MAX));
    assert_eq!(app.bitbake_setup.selected, 0);
    assert!(!app.bitbake_setup.loading);
}
