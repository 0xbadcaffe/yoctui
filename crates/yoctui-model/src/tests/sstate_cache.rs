use super::*;
fn app() -> App {
    let mut app = App::new(100, 4096);
    app.screen = Screen::Maintenance;
    app.focus = FocusTarget::Workspace;
    app.sstate_cache.dashboard = true;
    app.sstate_cache.daemon_supports_profiles = true;
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/build".into());
    app.workspace.bitbake_version = Some("2.20.0".into());
    app.build_environment = BuildEnvironmentState::Connected(BuildEnvironmentProfile {
        source_dir: "/source".into(),
        build_dir: "/build".into(),
        init_script: "/source/oe-init-build-env".into(),
    });
    app
}
fn settings() -> SstateCacheSettings {
    SstateCacheSettings {
        hash_server: "auto".into(),
        ..Default::default()
    }
}
#[test]
fn cache_settings_reject_injection_credentials_and_bad_templates() {
    for mirror in [
        "https://user:secret@host/cache",
        "https://host:0/cache",
        "https://host/PATH/foo",
        "https://host/PATH/PATH",
        "file:///../cache",
        "${CACHE}",
        "http://host/\nINJECT",
        "https://host/a;b",
    ] {
        assert!(
            SstateCacheSettings {
                mirror_url: mirror.into(),
                ..settings()
            }
            .validate()
            .is_err(),
            "{mirror}"
        );
    }
    for mirror in [
        "https://host/cache",
        "https://[::1]:8080/cache/PATH",
        "file:///mnt/cache",
    ] {
        assert!(
            SstateCacheSettings {
                mirror_url: mirror.into(),
                ..settings()
            }
            .validate()
            .is_ok(),
            "{mirror}"
        );
    }
    for hash in [
        "host",
        "host:0",
        "host:65536",
        "user@host:8080",
        "${HOST}",
        "host:abc",
        "unix:///../sock",
    ] {
        assert!(
            SstateCacheSettings {
                hash_server: hash.into(),
                ..Default::default()
            }
            .validate()
            .is_err(),
            "{hash}"
        );
    }
    for hash in [
        "auto",
        "host:8686",
        "[::1]:8686",
        "wss://host:443/hash",
        "unix:///run/hash.sock",
    ] {
        assert!(
            SstateCacheSettings {
                hash_server: hash.into(),
                ..Default::default()
            }
            .validate()
            .is_ok(),
            "{hash}"
        );
    }
    assert!(
        SstateCacheSettings {
            hash_server: "host:1234".into(),
            hash_upstream: "upstream:1234".into(),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        SstateCacheSettings {
            sstate_directory: "/".into(),
            ..settings()
        }
        .validate()
        .is_err()
    );
    assert!(
        SstateCacheSettings {
            downloads_directory: "/cache/../downloads".into(),
            ..settings()
        }
        .validate()
        .is_err()
    );
    assert!(SstateCacheSettings::default().validate().is_err());
    assert_eq!(
        SstateCacheSettings::parse(&settings().document()).unwrap(),
        settings()
    );
}
#[test]
fn cache_review_is_scoped_persistent_and_never_starts_a_build() {
    let mut app = app();
    let original = app.workspace.clone();
    assert_eq!(reduce(&mut app, SstateCacheAction::Configure), None);
    assert!(matches!(app.active_dialog(), Some(Dialog::SstateCache(_))));
    reduce(&mut app, SstateCacheAction::Review);
    app.workspace.build_dir = Some("/changed".into());
    assert_eq!(reduce(&mut app, SstateCacheAction::Save), None);
    assert!(app.preferences.sstate_cache_profiles.is_empty());
    app.workspace.build_dir = Some("/build".into());
    assert_eq!(
        reduce(&mut app, SstateCacheAction::Save),
        Some(Effect::PersistSettings)
    );
    assert_eq!(app.workspace, original);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.sstate_cache_profile().unwrap().settings, settings());
    app.workspace.build_dir = Some("/other-build".into());
    assert!(app.sstate_cache_profile().is_none());
    reduce(&mut app, SstateCacheAction::Disable);
    assert_eq!(app.preferences.sstate_cache_profiles.len(), 1);
    app.workspace.build_dir = Some("/build".into());
    reduce(&mut app, SstateCacheAction::Disable);
    assert!(app.preferences.sstate_cache_profiles.is_empty());
}
#[test]
fn cache_profiles_require_negotiated_support_and_scroll_reaches_final_row() {
    let mut app = app();
    reduce(&mut app, SstateCacheAction::Configure);
    reduce(&mut app, SstateCacheAction::Review);
    app.sstate_cache.daemon_supports_profiles = false;
    assert_eq!(reduce(&mut app, SstateCacheAction::Save), None);
    assert!(app.preferences.sstate_cache_profiles.is_empty());
    reduce(&mut app, SstateCacheAction::Cancel);
    app.workspace.variables.insert(
        "SSTATE_MIRRORS".into(),
        "file://.* https://user:secret@host/PATH".into(),
    );
    let rows = app.sstate_cache_rows();
    assert!(!rows.join("\n").contains("secret"));
    assert!(
        rows.iter()
            .any(|r| r.contains("No sstate summary reported"))
    );
    app.sstate_cache.visible_rows = 4;
    reduce(&mut app, SstateCacheAction::Last);
    assert_eq!(app.sstate_cache.scroll, rows.len() - 4);
    reduce(&mut app, SstateCacheAction::Scroll(-1));
    assert_eq!(app.sstate_cache.scroll, rows.len() - 5);
    reduce(&mut app, SstateCacheAction::First);
    assert_eq!(app.sstate_cache.scroll, 0);
}
#[test]
fn cache_menu_reuses_maintenance_without_duplicating_a_screen() {
    let mut app = app();
    app.screen = Screen::Dashboard;
    crate::update(&mut app, Action::SstateCache(SstateCacheAction::Open));
    assert_eq!(app.screen, Screen::Maintenance);
    assert_eq!(app.maintenance.view, MaintenanceView::Sstate);
    assert!(app.sstate_cache.dashboard);
    app.focus = FocusTarget::Workspace;
    reduce(&mut app, SstateCacheAction::Tools);
    assert!(!app.sstate_cache.dashboard);
}
#[test]
fn cache_preferences_reject_duplicate_or_excessive_profiles_and_review_scrolls() {
    let profile = SstateCacheProfile {
        build_directory: "/build".into(),
        settings: settings(),
    };
    let mut prefs = WorkbenchPreferences {
        sstate_cache_profiles: vec![profile.clone(), profile.clone()],
        ..Default::default()
    };
    assert!(prefs.validate().is_err());
    prefs.sstate_cache_profiles = (0..17)
        .map(|i| SstateCacheProfile {
            build_directory: format!("/build/{i}").into(),
            settings: settings(),
        })
        .collect();
    assert!(prefs.validate().is_err());
    let mut app = app();
    let long = SstateCacheProfile {
        settings: SstateCacheSettings {
            mirror_url: format!("https://host/{}", "segment/".repeat(150)),
            ..settings()
        },
        ..profile
    };
    app.sstate_cache.review_width = 60;
    app.sstate_cache.review_visible_rows = 12;
    let rows = long.review_rows(60);
    crate::open_dialog(
        &mut app,
        Dialog::SstateCache(Box::new(SstateCacheDialog::Review(long))),
    );
    reduce(&mut app, SstateCacheAction::Last);
    assert_eq!(app.sstate_cache.review_scroll, rows.len() - 12);
    reduce(&mut app, SstateCacheAction::Scroll(-1));
    assert_eq!(app.sstate_cache.review_scroll, rows.len() - 13);
    reduce(&mut app, SstateCacheAction::First);
    assert_eq!(app.sstate_cache.review_scroll, 0);
}
