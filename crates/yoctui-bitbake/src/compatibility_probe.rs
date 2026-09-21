include!("compatibility_probe/context_and_runner.rs");

include!("compatibility_probe/backend_probes.rs");

include!("compatibility_probe/observation_validation.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_executable;
    use std::{
        env,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{CapabilityCatalog, CapabilityId, IdentityAuthority, ToolIdentity};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn backend_probe_report_requires_exact_identity_and_unique_known_tokens() {
        let report = serde_json::json!({
            "schema": "yoctui.bridge-capability-probe.v1",
            "build_directory": "/build/romulus",
            "bitbake_version": "2.19.0",
            "capabilities": ["workspace", "build", "native_events"],
        });
        let parse = |value: &serde_json::Value| {
            parse_backend_capabilities(&value.to_string(), Path::new("/build/romulus"), "2.19.0")
        };
        assert_eq!(parse(&report).unwrap().len(), 3);
        for (key, value) in [
            ("schema", serde_json::json!("future")),
            ("build_directory", serde_json::json!("/other")),
            ("bitbake_version", serde_json::json!("2.18.0")),
            ("capabilities", serde_json::json!(["build", "build"])),
            ("capabilities", serde_json::json!(["invented"])),
            ("capabilities", serde_json::json!(vec!["build"; 65])),
        ] {
            let mut invalid = report.clone();
            invalid[key] = value;
            assert!(parse(&invalid).is_err(), "{invalid}");
        }
        let mut empty = report.clone();
        empty["capabilities"] = serde_json::json!([]);
        assert!(parse(&empty).unwrap().is_empty());
        empty["unexpected"] = serde_json::json!(true);
        assert!(parse(&empty).is_err());
        assert!(
            parse_backend_capabilities("not json", Path::new("/build/romulus"), "2.19.0").is_err()
        );
    }

    #[tokio::test]
    async fn backend_probe_process_ignores_stderr_and_fails_closed() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let report = serde_json::json!({
            "schema": "yoctui.bridge-capability-probe.v1",
            "build_directory": fixture.root,
            "bitbake_version": "99.0",
            "capabilities": ["build", "native_events"],
        });
        let environment = BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]);
        write_executable(
            &fixture.tool,
            &format!("#!/bin/sh\nprintf '%s\\n' '{report}'\nprintf 'probe diagnostic\\n' >&2\n"),
        );
        let capabilities =
            probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
                .await
                .unwrap();
        assert!(capabilities.contains("build"));
        assert!(
            probe_bundled_backend_capabilities(
                &fixture.tool,
                &fixture.root,
                &environment,
                "2.19.0"
            )
            .await
            .is_err()
        );
        write_executable(
            &fixture.tool,
            &format!("#!/bin/sh\nprintf '%s\\n' '{report}' >&2\n"),
        );
        assert!(
            probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
                .await
                .is_err()
        );
        write_executable(&fixture.tool, "#!/bin/sh\nprintf '%70000s' x\n");
        assert!(
            probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
                .await
                .is_err()
        );
        let overridden = BTreeMap::from([(
            "YOCTUI_BRIDGE_PATH".into(),
            fixture.tool.to_string_lossy().into_owned(),
        )]);
        assert!(
            probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &overridden, "99.0")
                .await
                .unwrap_err()
                .contains("custom bridge")
        );
    }

    struct Fixture {
        root: PathBuf,
        tool: PathBuf,
    }

    impl Fixture {
        fn new(body: &str) -> Self {
            Self::new_tool(CapabilityToolId::Devtool, body)
        }

        fn new_tool(tool_id: CapabilityToolId, body: &str) -> Self {
            let root = env::temp_dir().join(format!(
                "yoctui-compat-probe-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let tool = root.join(tool_id.executable_name());
            write_executable(&tool, body);
            Self { root, tool }
        }

        fn context(&self) -> CapabilityProbeContext {
            self.context_for(CapabilityToolId::Devtool)
        }

        fn context_for(&self, tool_id: CapabilityToolId) -> CapabilityProbeContext {
            let identity = YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    self.root.clone(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: tool_id.executable_name().into(),
                        executable: self.tool.clone(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            };
            CapabilityProbeContext::new(
                identity,
                self.root.clone(),
                BTreeMap::from([(tool_id, self.tool.clone())]),
                BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
                Some(BTreeSet::from(["create_spdx".into()])),
                Some(BTreeSet::from(["MACHINE".into()])),
                Some(BTreeSet::from(["getvar".into()])),
                Some(BTreeSet::from(["state_snapshots".into()])),
                Some(BTreeSet::from(["wic".into()])),
                Some(BTreeSet::from(["buildhistory".into()])),
            )
            .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn compatibility_probe_uses_exact_shell_free_help_and_version_argv() {
        let fixture = Fixture::new(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\ncase \"$*\" in\n  *--version*) echo 'devtool 1.0' ;;\n  *) echo 'modify upgrade --force' ;;\nesac\n",
        );
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        let help = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandHelp {
                    tool: CapabilityToolId::Devtool,
                    subcommand: Some("upgrade".into()),
                },
            )
            .await;
        assert_eq!(help.status, CapabilityProbeStatus::Positive);
        assert_eq!(help.evidence.argv[1..], ["upgrade", "--help"]);
        let version = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(version.status, CapabilityProbeStatus::Positive);
        assert_eq!(version.evidence.argv[1..], ["--version"]);
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
            "upgrade\n--help\n--version\n"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn compatibility_probe_background_priority_reaches_the_tool_process() {
        let fixture = Fixture::new(
            "#!/bin/sh\nsleep 0.1\nps -o ni= -p $$ > probe.nice\necho 'devtool 1.0'\n",
        );
        let observation = CapabilityProbeRunner::default()
            .with_background_priority()
            .probe(
                &fixture.context(),
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(observation.status, CapabilityProbeStatus::Positive);
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.nice"))
                .unwrap()
                .trim(),
            "10"
        );
    }

    #[tokio::test]
    async fn compatibility_probe_distinguishes_missing_tool_command_and_option() {
        let fixture =
            Fixture::new("#!/bin/sh\nif [ \"$1\" = bad ]; then exit 2; fi\necho 'modify finish'\n");
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        let missing_tool = runner
            .probe(
                &context,
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::Wic,
                },
            )
            .await;
        assert_eq!(missing_tool.status, CapabilityProbeStatus::Negative);
        let command = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandHelp {
                    tool: CapabilityToolId::Devtool,
                    subcommand: Some("bad".into()),
                },
            )
            .await;
        assert_eq!(command.status, CapabilityProbeStatus::Negative);
        let option = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandOption {
                    tool: CapabilityToolId::Devtool,
                    subcommand: None,
                    option: "--force".into(),
                },
            )
            .await;
        assert_eq!(option.status, CapabilityProbeStatus::Negative);
    }

    #[tokio::test]
    async fn compatibility_probe_bitbake_getvar_uses_exact_initialized_tool_and_options() {
        let fixture = Fixture::new_tool(
            CapabilityToolId::BitBakeGetVar,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\necho 'usage: bitbake-getvar [--value] [-r RECIPE] variable'\necho '  -r, --recipe RECIPE'\n",
        );
        let context = fixture.context_for(CapabilityToolId::BitBakeGetVar);
        let runner = CapabilityProbeRunner::default();
        for probe in [
            CapabilityProbeSpec::Executable {
                tool: CapabilityToolId::BitBakeGetVar,
            },
            CapabilityProbeSpec::CommandHelp {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
            },
            CapabilityProbeSpec::CommandOption {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
                option: "--value".into(),
            },
            CapabilityProbeSpec::CommandOption {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
                option: "--recipe".into(),
            },
        ] {
            assert_eq!(
                runner.probe(&context, &probe).await.status,
                CapabilityProbeStatus::Positive
            );
        }
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
            "--help\n--help\n--help\n"
        );

        let missing_context = Fixture::new("#!/bin/sh\necho ok\n").context();
        assert_eq!(
            runner
                .probe(
                    &missing_context,
                    &CapabilityProbeSpec::Executable {
                        tool: CapabilityToolId::BitBakeGetVar
                    }
                )
                .await
                .status,
            CapabilityProbeStatus::Negative
        );
    }

    #[tokio::test]
    async fn raw_capability_probe_is_shell_free_and_distinguishes_direct_evidence() {
        let fixture = Fixture::new_tool(
            CapabilityToolId::BitBake,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\necho 'usage: bitbake --dry-run --runall'\n",
        );
        let context = fixture.context_for(CapabilityToolId::BitBake);
        let runner = CapabilityProbeRunner::default();
        let catalog = CapabilityCatalog::builtin();

        let positive = runner
            .probe(
                &context,
                &catalog
                    .entry(CapabilityId::BitBakeRawDryRun)
                    .unwrap()
                    .probes[0],
            )
            .await;
        assert_eq!(positive.status, CapabilityProbeStatus::Positive);
        assert_eq!(positive.evidence.argv[1..], ["--help"]);

        let negative = runner
            .probe(
                &context,
                &catalog
                    .entry(CapabilityId::BitBakeRawEventLog)
                    .unwrap()
                    .probes[0],
            )
            .await;
        assert_eq!(negative.status, CapabilityProbeStatus::Negative);
        assert_eq!(negative.evidence.argv[1..], ["--help"]);
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
            "--help\n--help\n"
        );

        let oversized = Fixture::new_tool(
            CapabilityToolId::BitBake,
            "#!/bin/sh\nprintf '%080d\\n' 0\n",
        );
        let inconclusive = CapabilityProbeRunner::with_limits(Duration::from_secs(1), 16)
            .unwrap()
            .probe(
                &oversized.context_for(CapabilityToolId::BitBake),
                &catalog.entry(CapabilityId::BitBakeRawCli).unwrap().probes[0],
            )
            .await;
        assert_eq!(inconclusive.status, CapabilityProbeStatus::Inconclusive);
    }

    #[tokio::test]
    async fn compatibility_probe_reports_timeout_oversize_and_stale_executable_as_inconclusive() {
        let timeout_fixture = Fixture::new("#!/bin/sh\nsleep 30\n");
        let runner = CapabilityProbeRunner::with_limits(Duration::from_millis(30), 128).unwrap();
        let timed_out = runner
            .probe(
                &timeout_fixture.context(),
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(timed_out.status, CapabilityProbeStatus::Inconclusive);
        assert!(timed_out.evidence.detail.contains("timed out"));

        let large_fixture = Fixture::new("#!/bin/sh\nyes x | head -c 4096\n");
        let output_bound_runner =
            CapabilityProbeRunner::with_limits(Duration::from_secs(1), 128).unwrap();
        let oversized = output_bound_runner
            .probe(
                &large_fixture.context(),
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(oversized.status, CapabilityProbeStatus::Inconclusive);
        assert!(oversized.evidence.detail.contains("safety bound"));

        let stale_fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let context = stale_fixture.context();
        fs::remove_file(&stale_fixture.tool).unwrap();
        let stale = CapabilityProbeRunner::default()
            .probe(
                &context,
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(stale.status, CapabilityProbeStatus::Inconclusive);
    }

    #[test]
    fn compatibility_probe_spawn_retry_classifies_only_text_file_busy_as_transient() {
        assert!(is_transient_probe_spawn_error(
            &io::Error::from_raw_os_error(libc::ETXTBSY)
        ));
        assert!(!is_transient_probe_spawn_error(
            &io::Error::from_raw_os_error(libc::ENOENT)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn compatibility_probe_accepts_safe_same_directory_utility_symlink_without_losing_argv0()
    {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new("#!/bin/sh\necho modify\n");
        let target = fixture.root.join("devtool-real");
        fs::rename(&fixture.tool, &target).unwrap();
        symlink("devtool-real", &fixture.tool).unwrap();
        let observation = CapabilityProbeRunner::default()
            .probe(
                &fixture.context(),
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(observation.status, CapabilityProbeStatus::Positive);
        assert_eq!(
            observation.evidence.argv,
            [fixture.tool.display().to_string()]
        );
    }

    #[tokio::test]
    async fn compatibility_probe_uncollected_inventory_is_inconclusive_not_negative() {
        let fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let mut context = fixture.context();
        context.metadata_tasks = None;
        context.metadata_variables = None;
        context.backend_capabilities = None;
        context.configurations = None;
        for probe in [
            CapabilityProbeSpec::MetadataAnyTask {
                names: vec!["do_build".into()],
            },
            CapabilityProbeSpec::MetadataVariable {
                name: "MACHINE".into(),
            },
            CapabilityProbeSpec::BackendCapability {
                name: "workspace".into(),
            },
            CapabilityProbeSpec::Configuration {
                name: "ptest_enabled".into(),
            },
        ] {
            assert_eq!(
                CapabilityProbeRunner::default()
                    .probe(&context, &probe)
                    .await
                    .status,
                CapabilityProbeStatus::Inconclusive
            );
        }
    }

    #[tokio::test]
    async fn compatibility_probe_maps_typed_non_process_observations() {
        let fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        for probe in [
            CapabilityProbeSpec::MetadataAnyTask {
                names: vec!["create_spdx".into()],
            },
            CapabilityProbeSpec::MetadataVariable {
                name: "MACHINE".into(),
            },
            CapabilityProbeSpec::BackendCapability {
                name: "getvar".into(),
            },
            CapabilityProbeSpec::ProtocolCapability {
                name: "state_snapshots".into(),
            },
            CapabilityProbeSpec::Artifact { kind: "wic".into() },
            CapabilityProbeSpec::Configuration {
                name: "buildhistory".into(),
            },
        ] {
            assert_eq!(
                runner.probe(&context, &probe).await.status,
                CapabilityProbeStatus::Positive
            );
        }
        assert_eq!(
            runner
                .probe(
                    &context,
                    &CapabilityProbeSpec::MetadataVariable {
                        name: "ABSENT".into()
                    },
                )
                .await
                .status,
            CapabilityProbeStatus::Negative
        );
    }

    #[test]
    fn compatibility_probe_context_rejects_environment_and_tool_mismatch() {
        let fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let mut identity = fixture.context().environment().clone();
        identity.build_directory = AuthoritativeValue::detected(
            "/other/build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        let result = CapabilityProbeContext::new(
            identity,
            fixture.root.clone(),
            BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
            BTreeMap::new(),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
        );
        assert!(matches!(
            result,
            Err(CapabilityProbeContextError::EnvironmentMismatch)
        ));

        let mut identity = fixture.context().environment().clone();
        identity.available_tools = AuthoritativeValue::detected(
            vec![ToolIdentity {
                id: "devtool".into(),
                executable: "/other/devtool".into(),
                version: None,
            }],
            IdentityAuthority::ExecutableProbe,
        );
        let result = CapabilityProbeContext::new(
            identity,
            fixture.root.clone(),
            BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
            BTreeMap::new(),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
        );
        assert!(matches!(
            result,
            Err(CapabilityProbeContextError::EnvironmentMismatch)
        ));
    }
}
