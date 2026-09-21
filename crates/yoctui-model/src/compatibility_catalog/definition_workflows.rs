fn definition_workflows(id: CapabilityId) -> Option<Definition> {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    Some(match id {
        Id::WicCreate => tool_command(
            "Wic image creation",
            Tool::Wic,
            Some("create"),
            &[],
            "wic.create.argv",
        ),
        Id::RunQemu => tool_command("runqemu launch", Tool::Runqemu, None, &[], "runqemu.argv"),
        Id::SdkPopulate => task(
            "standard SDK population",
            &["populate_sdk"],
            "bitbake.populate_sdk",
        ),
        Id::SdkExtensible => task(
            "extensible SDK population",
            &["populate_sdk_ext"],
            "bitbake.populate_sdk_ext",
        ),
        Id::SdkPublish => tool_command(
            "SDK publication",
            Tool::OePublishSdk,
            None,
            &[],
            "oe_publish_sdk.argv",
        ),
        Id::SdkNativeTools => tool_command(
            "native SDK tool execution",
            Tool::OeFindNativeSysroot,
            None,
            &[],
            "oe_find_native_sysroot.argv",
        ),
        Id::CveCheck => task("CVE checking", &["cve_check"], "bitbake.cve_check"),
        Id::SpdxCreate => task(
            "SPDX creation",
            &["create_spdx", "create_recipe_sbom", "create_rootfs_sbom"],
            "bitbake.spdx",
        ),
        Id::YoctoCheckLayer => tool_command(
            "Yocto layer checking",
            Tool::YoctoCheckLayer,
            None,
            &[],
            "yocto_check_layer.argv",
        ),
        Id::ResultTool => tool_command(
            "resulttool operations",
            Tool::Resulttool,
            None,
            &[],
            "resulttool.argv",
        ),
        Id::OeSelftest => tool_command(
            "OpenEmbedded selftest",
            Tool::OeSelftest,
            None,
            &[],
            "oe_selftest.argv",
        ),
        Id::BitBakeSelftest => tool_command(
            "BitBake selftest",
            Tool::BitBakeSelftest,
            None,
            &[],
            "bitbake_selftest.argv",
        ),
        Id::TestImage => task("runtime image testing", &["testimage"], "bitbake.testimage"),
        Id::TestSdk => task("standard SDK testing", &["testsdk"], "bitbake.testsdk"),
        Id::TestSdkExtensible => task(
            "extensible SDK testing",
            &["testsdkext"],
            "bitbake.testsdkext",
        ),
        Id::Ptest => (
            "installed ptest execution",
            vec![Tool::BitBake],
            Vec::new(),
            vec![MetadataRequirement::Configuration {
                name: "ptest_enabled".into(),
            }],
            vec![CapabilityProbeSpec::Configuration {
                name: "ptest_enabled".into(),
            }],
            implementation("bitbake.ptest", Kind::MetadataTask),
            None,
        ),
        Id::QaTask => (
            "configured QA task execution",
            vec![Tool::BitBake],
            Vec::new(),
            vec![MetadataRequirement::Configuration {
                name: "qa_tasks".into(),
            }],
            vec![CapabilityProbeSpec::Configuration {
                name: "qa_tasks".into(),
            }],
            implementation("bitbake.qa_task", Kind::MetadataTask),
            None,
        ),
        Id::MenuConfig => task("menuconfig", &["menuconfig"], "bitbake.menuconfig"),
        Id::DevShell => task("development shell", &["devshell"], "bitbake.devshell"),
        Id::BuildHistory => task(
            "build history",
            &["buildhistory_get_image_installed"],
            "bitbake.buildhistory",
        ),
        Id::BuildHistoryCompare => tool_command(
            "build history comparison",
            Tool::BuildHistoryDiff,
            None,
            &[],
            "buildhistory_diff.argv",
        ),
        Id::LockedSignatures => task(
            "locked signatures",
            &["locked_sigs"],
            "bitbake.locked_signatures",
        ),
        Id::HashservDiagnostics => {
            let mut value = backend(
                "hash equivalence server diagnostics",
                "hashserv",
                "bitbake.hashserv_diagnostics",
            );
            value.3.push(MetadataRequirement::Variable {
                name: "BB_HASHSERVE".into(),
            });
            value.4.push(CapabilityProbeSpec::MetadataVariable {
                name: "BB_HASHSERVE".into(),
            });
            value
        }
        Id::PrservDiagnostics => {
            let mut value = backend(
                "PR service diagnostics",
                "prserv",
                "bitbake.prserv_diagnostics",
            );
            value.3.push(MetadataRequirement::Variable {
                name: "PRSERV_HOST".into(),
            });
            value.4.push(CapabilityProbeSpec::MetadataVariable {
                name: "PRSERV_HOST".into(),
            });
            value
        }
        Id::SstateReadiness => tool_command(
            "shared-state readiness inspection",
            Tool::OeCheckSstate,
            None,
            &[],
            "oe_check_sstate.argv",
        ),
        Id::SstateCleanup => tool_command(
            "shared-state cache cleanup",
            Tool::SstateCacheManagement,
            None,
            &[],
            "sstate_cache_management.argv",
        ),
        Id::PrservManagement => tool_command(
            "PR service management",
            Tool::BitBakePrserv,
            None,
            &[],
            "bitbake_prserv.argv",
        ),
        Id::BuildCompare => tool_command(
            "build output comparison",
            Tool::BuildCompare,
            None,
            &[],
            "build_compare.argv",
        ),
        Id::GitArchive => tool_command(
            "OpenEmbedded Git archive",
            Tool::OeGitArchive,
            None,
            &[],
            "oe_git_archive.argv",
        ),
        _ => return None,
    })
}
