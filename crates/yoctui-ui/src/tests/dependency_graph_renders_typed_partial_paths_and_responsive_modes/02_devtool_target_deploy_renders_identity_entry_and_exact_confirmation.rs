#[test]
fn devtool_target_deploy_renders_identity_entry_and_exact_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    app.dialogs
        .push_back(Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
            identity: identity.clone(),
            target: "qemuarm".into(),
        }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Deploy build with SSH/SCP"));
    assert!(output.contains("busybox.bb"));
    assert!(output.contains("qemuarm"));

    app.dialogs.clear();
    app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
        yoctui_model::DevtoolDeployPlan {
            identity,
            target: "qemuarm".into(),
        },
    ));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm SSH/SCP deployment"));
    assert!(output.contains("devtool deploy-target busybox qemuarm"));
    assert!(output.contains("busybox.bb"));
}

#[test]
fn devtool_undeploy_and_upgrade_render_exact_distinct_commands() {
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/busybox.bb".into(),
    };
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::DevtoolUndeployConfirmation(
        yoctui_model::DevtoolUndeployPlan {
            identity: identity.clone(),
            target: "root@board".into(),
        },
    ));
    let undeploy = rendered_text(&app, 120, 25);
    assert!(
        undeploy.contains("devtool undeploy-target busybox root@board"),
        "{undeploy}"
    );

    app.dialogs.clear();
    app.dialogs.push_back(Dialog::DevtoolUpgradeConfirmation(
        yoctui_model::DevtoolUpgradePlan { identity },
    ));
    let upgrade = rendered_text(&app, 120, 25);
    assert!(upgrade.contains("devtool upgrade busybox"), "{upgrade}");
}
#[test]
fn devwork_editor_renders_confirmation_and_workspace_editor_build_shortcut() {
    let mut confirmation = App::new(10, 1_000);
    confirmation
        .dialogs
        .push_back(Dialog::DevtoolModifyConfirmation(
            yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            },
        ));
    let output = rendered_text(&confirmation, 120, 30);
    assert!(output.contains("Confirm Devtool modify"), "{output}");
    assert!(output.contains("devtool modify busybox"), "{output}");
    assert!(output.contains("busybox.bb"), "{output}");

    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "busybox".into(),
        root: "/build/workspace/sources/busybox".into(),
        files: vec!["main.c".into()],
        file_inventory_truncated: false,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main() {}".into()),
        searching: false,
    }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Workspace file tree: busybox"));
    assert!(output.contains("int main() {}"));
    assert!(output.contains("Ctrl+B build recipe"));
}
#[test]
fn devwork_terminal_renders_destination_authority_and_zero_spawn_cancel_hint() {
    let mut app = App::new(10, 1_000);
    app.detached_terminal = yoctui_model::DetachedTerminalAvailability::Available {
        launcher: "x-terminal-emulator".into(),
    };
    app.dialogs
        .push_back(Dialog::TerminalLaunch(yoctui_model::TerminalLaunchDialog {
            request: yoctui_model::TerminalLaunchRequest {
                name: "devshell:busybox".into(),
                kind: yoctui_model::TerminalCreationKind::Devshell,
                cwd: "/work/build".into(),
                program: "/usr/bin/env".into(),
                arguments: vec![
                    "bitbake".into(),
                    "busybox".into(),
                    "-c".into(),
                    "devshell".into(),
                ],
            },
            destination: yoctui_model::TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let output = rendered_text(&app, 120, 30);
    assert!(output.contains("Choose terminal destination"), "{output}");
    assert!(output.contains("Embedded in Yoctui"), "{output}");
    assert!(output.contains("x-terminal-emulator"), "{output}");
    assert!(output.contains("bitbake busybox -c devshell"), "{output}");
    assert!(output.contains("cancel without spawning"), "{output}");
}
#[test]
fn ux_list_tree_layer_browser_renders_external_state_and_numbered_preview() {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.workspace.layers.push(yoctui_model::Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(7),
    });
    app.layer_relationships = Some(yoctui_model::LayerRelationships {
        layers: vec![yoctui_model::LayerRelationship {
            name: "meta-demo".into(),
            compatible: vec!["scarthgap".into()],
            ..yoctui_model::LayerRelationship::default()
        }],
    });
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries = vec![yoctui_model::LayerBrowserEntry {
        path: "/layers/meta-demo/conf/layer.conf".into(),
        is_dir: false,
        size: Some(31),
        git: yoctui_model::GitFileState::Modified,
        ..yoctui_model::LayerBrowserEntry::default()
    }];
    browser.preview = "BBFILE_COLLECTIONS += \\\"demo\\\"".into();
    browser.preview_kind = yoctui_model::PreviewKind::Text;
    app.layer_browser = Some(browser);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Configured layers"));
    assert!(output.contains("meta-demo"));
    assert!(output.contains("hidden off"));
    assert!(output.contains("layer.conf"));
    assert!(output.contains("BBFILE_COLLECTIONS"));
    assert!(output.contains("M"));
    assert!(output.contains("1"));
}

#[test]
fn layer_browser_gives_unused_tree_width_to_the_file_preview() {
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries.push(LayerBrowserEntry {
        path: "/layers/meta-demo/conf/layer.conf".into(),
        depth: 1,
        ..LayerBrowserEntry::default()
    });

    let compact = layer_browser_left_width(&browser, 140);
    assert_eq!(compact, 38);
    assert_eq!(140 - compact, 102);

    browser.entries[0].path =
        "/layers/meta-demo/recipes-core/example/a-very-long-recipe-filename.bb".into();
    let expanded = layer_browser_left_width(&browser, 140);
    assert!(expanded > compact);
    assert!(expanded <= 54);
    assert!(140 - expanded >= 86);
}

#[test]
fn ux_layer_browser_tui_tree_widget_preserves_model_identity_viewport_and_ascii() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.workspace.layers.push(yoctui_model::Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(7),
    });
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    let conf = PathBuf::from("/layers/meta-demo/conf");
    browser.expanded.insert(conf.clone());
    browser.entries = vec![
        LayerBrowserEntry {
            path: conf.clone(),
            is_dir: true,
            git: GitFileState::Clean,
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: conf.join("layer.conf"),
            depth: 1,
            git: GitFileState::Modified,
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: "/layers/meta-demo/recipes-core".into(),
            is_dir: true,
            git: GitFileState::Untracked,
            ..LayerBrowserEntry::default()
        },
    ];
    browser.selection = 1;

    let indexed = browser.entries.iter().enumerate().collect::<Vec<_>>();
    let projection = layer_tree_widget_projection(&browser, &indexed, true, false).unwrap();
    let mut state = TreeState::default();
    for path in &projection.opened {
        state.open(path.clone());
    }
    state.select(projection.selected.clone().unwrap());
    let flattened = state.flatten(&projection.items);
    assert_eq!(flattened.len(), browser.entries.len());
    assert_eq!(
        state.selected().last(),
        Some(&PathBuf::from("/layers/meta-demo/conf/layer.conf"))
    );
    assert!(
        flattened[1]
            .identifier
            .starts_with(&[conf.clone(), conf.join("layer.conf")])
    );

    app.layer_browser = Some(browser.clone());
    let unicode = rendered_text(&app, 120, 30);
    assert!(unicode.contains("▾ conf/"), "{unicode}");
    assert!(unicode.contains("layer.conf M"), "{unicode}");
    assert!(unicode.contains("▸ recipes-core/ ?"), "{unicode}");

    app.preferences.symbols = SymbolPreference::Ascii;
    app.color_enabled = false;
    let ascii = rendered_text(&app, 120, 30);
    assert!(ascii.contains("- conf/"), "{ascii}");
    assert!(ascii.contains("+ recipes-core/ ?"), "{ascii}");

    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries = (0..40)
        .map(|index| LayerBrowserEntry {
            path: format!("/layers/meta-demo/file-{index:02}.bb").into(),
            ..LayerBrowserEntry::default()
        })
        .collect();
    browser.selection = 39;
    let bottom = rendered_text(&app, 120, 30);
    assert!(bottom.contains("file-39.bb"), "{bottom}");
    assert!(!bottom.contains("file-00.bb"), "{bottom}");

    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries = vec![
        LayerBrowserEntry {
            path: "/layers/meta-demo/duplicate".into(),
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: "/layers/meta-demo/duplicate".into(),
            ..LayerBrowserEntry::default()
        },
    ];
    browser.selection = 0;
    let malformed = rendered_text(&app, 120, 30);
    assert!(malformed.contains("Layer tree unavailable"), "{malformed}");
}

#[test]
fn layer_tree_binary_preview_and_responsive_modes_never_render_bytes() {
    for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Layers;
        app.focus = FocusTarget::Workspace;
        let mut browser = LayerBrowser::new("meta-binary".into(), "/layers/meta-binary".into());
        browser.entries.push(yoctui_model::LayerBrowserEntry {
            path: "/layers/meta-binary/image.bin".into(),
            size: Some(100_000),
            git: yoctui_model::GitFileState::Unavailable,
            ..yoctui_model::LayerBrowserEntry::default()
        });
        browser.preview = "\0secret".into();
        browser.preview_kind = yoctui_model::PreviewKind::Binary;
        browser.preview_truncated = true;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!output.contains("secret"));
        if width >= 80 && height >= 24 {
            assert!(output.contains("Binary preview unavailable"));
        }
    }
}
#[test]
fn bitbake_preview_highlights_assignments_and_comments() {
    let app = App::new(10, 1_000);
    let preview = source_preview("SUMMARY = \"demo\" # explanation", "demo.bb", &app);
    assert_eq!(
        preview.lines[0].spans[0].style.fg,
        Some(Color::Rgb(226, 170, 0))
    );
    assert_eq!(
        preview.lines[0].spans[1].style.fg,
        Some(Color::Rgb(176, 126, 214))
    );
    assert_eq!(
        preview.lines[0].spans[2].style.fg,
        Some(Color::Rgb(139, 211, 0))
    );
    assert_eq!(
        preview.lines[0].spans[3].style.fg,
        Some(Color::Rgb(155, 166, 172))
    );
}

#[test]
fn device_tree_preview_highlights_directives_nodes_properties_values_and_comments() {
    let app = App::new(10, 1_000);
    let preview = source_preview(
        "#include \"soc.dtsi\"\n/dts-v1/;\nuart0: serial@1000 {\n  compatible = \"http://vendor/\\\"device\"; // UART\n  interrupt-controller;\n  /* retained */ status = \"okay\";\n};",
        "board.dts",
        &app,
    );
    let styled = preview
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter_map(|span| span.style.fg.map(|color| (span.content.as_ref(), color)))
        .collect::<Vec<_>>();

    assert!(styled.iter().any(|(text, _)| *text == "/dts-v1/"));
    assert!(styled.iter().any(|(text, _)| *text == "#include"));
    assert!(styled.iter().any(|(text, _)| *text == "uart0"));
    assert!(styled.iter().any(|(text, _)| *text == "compatible"));
    assert!(
        styled
            .iter()
            .any(|(text, _)| *text == "\"http://vendor/\\\"device\"")
    );
    assert!(
        styled
            .iter()
            .any(|(text, _)| *text == "interrupt-controller")
    );
    assert!(
        styled
            .iter()
            .any(|(text, color)| { *text == "// UART" && *color == Color::Rgb(155, 166, 172) })
    );
    assert!(
        styled.iter().any(|(text, color)| {
            *text == "/* retained */" && *color == Color::Rgb(155, 166, 172)
        })
    );
}

#[test]
fn device_tree_compile_dialog_renders_typed_options_and_derived_paths() {
    let mut app = App::new(10, 1_000);
    app.dialogs
        .push_back(Dialog::DtcCompile(yoctui_model::DtcCompileDialog::new(
            yoctui_model::PlatformComponent::Kernel,
            &yoctui_model::PlatformFile {
                path: "/workspace/kernel/board.dts".into(),
                root: "/workspace/kernel".into(),
                kind: yoctui_model::PlatformFileKind::Dts,
                size_bytes: 64,
            },
            "/toolchain/bin/dtc".into(),
        )));

    let output = rendered_text(&app, 120, 30);
    for expected in [
        "Compile device tree",
        "board.dts",
        "board.yoctui.dtb",
        "Generate symbols (-@)",
        "Stable sort (-s)",
        "Output padding (-p)",
        "Reserve entries (-R)",
        "Enter review launch",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert!(rendered_text(&app, 80, 24).contains("Compile device tree"));
    if let Some(Dialog::DtcCompile(dialog)) = app.dialogs.front_mut() {
        dialog.source = PathBuf::from(format!("/workspace/{}/board.dts", "nested/".repeat(30)));
        dialog.output = PathBuf::from(format!(
            "/workspace/{}/board.yoctui.dtb",
            "nested/".repeat(30)
        ));
    }
    assert!(rendered_text(&app, 80, 24).contains("Enter review launch"));
    let _ = rendered_text(&app, 40, 10);
}
