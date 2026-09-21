fn definition_raw(id: CapabilityId) -> Option<Definition> {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    Some(match id {
        Id::BitBakeRawCli => (
            "Raw BitBake CLI",
            vec![Tool::BitBake],
            vec![command(Tool::BitBake, None, &[])],
            Vec::new(),
            vec![help(Tool::BitBake, None)],
            implementation("bitbake.raw.argv", Kind::Command),
            None,
        ),
        Id::BitBakeRawShowVersions => bitbake_option(
            "Raw show-versions option",
            "--show-versions",
            "bitbake.raw.show_versions.argv",
        ),
        Id::BitBakeRawTaskExecution => bitbake_option(
            "Raw task execution option",
            "--cmd",
            "bitbake.raw.task_execution.argv",
        ),
        Id::BitBakeRawClearStamp => bitbake_option(
            "Raw clear-stamp option",
            "--clear-stamp",
            "bitbake.raw.clear_stamp.argv",
        ),
        Id::BitBakeRawDryRun => bitbake_option(
            "Raw dry-run option",
            "--dry-run",
            "bitbake.raw.dry_run.argv",
        ),
        Id::BitBakeRawParseOnly => bitbake_option(
            "Raw parse-only option",
            "--parse-only",
            "bitbake.raw.parse_only.argv",
        ),
        Id::BitBakeRawContinue => bitbake_option(
            "Raw continue option",
            "--continue",
            "bitbake.raw.continue.argv",
        ),
        Id::BitBakeRawProfile => bitbake_option(
            "Raw profile option",
            "--profile",
            "bitbake.raw.profile.argv",
        ),
        Id::BitBakeRawDumpSignatures => bitbake_option(
            "Raw dump-signatures option",
            "--dump-signatures",
            "bitbake.raw.dump_signatures.argv",
        ),
        Id::BitBakeRawRevisionsChanged => bitbake_option(
            "Raw revisions-changed option",
            "--revisions-changed",
            "bitbake.raw.revisions_changed.argv",
        ),
        Id::BitBakeRawBuildFile => bitbake_option(
            "Raw buildfile option",
            "--buildfile",
            "bitbake.raw.buildfile.argv",
        ),
        Id::BitBakeRawDebug => {
            bitbake_option("Raw debug option", "--debug", "bitbake.raw.debug.argv")
        }
        Id::BitBakeRawLogDomains => bitbake_option(
            "Raw log-domains option",
            "--log-domains",
            "bitbake.raw.log_domains.argv",
        ),
        Id::BitBakeRawVerbose => bitbake_option(
            "Raw verbose option",
            "--verbose",
            "bitbake.raw.verbose.argv",
        ),
        Id::BitBakeRawQuiet => {
            bitbake_option("Raw quiet option", "--quiet", "bitbake.raw.quiet.argv")
        }
        Id::BitBakeRawEventLog => bitbake_option(
            "Raw event-log option",
            "--write-log",
            "bitbake.raw.event_log.argv",
        ),
        Id::BitBakeRawUi => bitbake_option("Raw UI option", "--ui", "bitbake.raw.ui.argv"),
        Id::BitBakeRawServerBind => bitbake_option(
            "Raw server bind option",
            "--bind",
            "bitbake.raw.server_bind.argv",
        ),
        Id::BitBakeRawServerIdleTimeout => bitbake_option(
            "Raw server idle-timeout option",
            "--idle-timeout",
            "bitbake.raw.server_idle_timeout.argv",
        ),
        Id::BitBakeRawServerRemote => bitbake_option(
            "Raw remote-server option",
            "--remote-server",
            "bitbake.raw.server_remote.argv",
        ),
        Id::BitBakeRawServerToken => bitbake_option(
            "Raw server token option",
            "--token",
            "bitbake.raw.server_token.argv",
        ),
        Id::BitBakeRawServerObserve => bitbake_option(
            "Raw observe-only option",
            "--observe-only",
            "bitbake.raw.server_observe.argv",
        ),
        Id::BitBakeRawConfigRead => bitbake_option(
            "Raw pre-configuration option",
            "--read",
            "bitbake.raw.config_read.argv",
        ),
        Id::BitBakeRawConfigPostRead => bitbake_option(
            "Raw post-configuration option",
            "--postread",
            "bitbake.raw.config_postread.argv",
        ),
        Id::BitBakeRawIgnoreDeps => bitbake_option(
            "Raw ignore-dependencies option",
            "--ignore-deps",
            "bitbake.raw.ignore_deps.argv",
        ),
        Id::BitBakeRawMulticonfig => (
            "Raw multiconfig target syntax",
            vec![Tool::BitBake],
            vec![command(Tool::BitBake, None, &[])],
            Vec::new(),
            vec![CapabilityProbeSpec::CommandHelpText {
                tool: Tool::BitBake,
                needle: "mc:".into(),
            }],
            implementation("bitbake.raw.multiconfig.argv", Kind::Command),
            None,
        ),
        Id::BitBakeRawRunAll => {
            bitbake_option("Raw runall option", "--runall", "bitbake.raw.runall.argv")
        }
        Id::BitBakeRawRunOnly => bitbake_option(
            "Raw runonly option",
            "--runonly",
            "bitbake.raw.runonly.argv",
        ),
        Id::BitBakeRawNoSetscene => bitbake_option(
            "Raw no-setscene option",
            "--no-setscene",
            "bitbake.raw.no_setscene.argv",
        ),
        Id::BitBakeRawSkipSetscene => bitbake_option(
            "Raw skip-setscene option",
            "--skip-setscene",
            "bitbake.raw.skip_setscene.argv",
        ),
        Id::BitBakeRawSetsceneOnly => bitbake_option(
            "Raw setscene-only option",
            "--setscene-only",
            "bitbake.raw.setscene_only.argv",
        ),
        _ => return None,
    })
}
