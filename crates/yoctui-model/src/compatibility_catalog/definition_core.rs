fn definition_core(id: CapabilityId) -> Option<Definition> {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    Some(match id {
        Id::BitBakeWorkspaceInspection => backend_with_version_fallback(
            "BitBake workspace inspection",
            "workspace",
            "tinfoil.workspace",
        ),
        Id::BitBakeRecipeInventory => {
            backend_with_version_fallback("BitBake recipe inventory", "recipes", "tinfoil.recipes")
        }
        Id::BitBakeRecipeDependencies => backend_with_version_fallback(
            "BitBake recipe dependencies",
            "recipe_dependencies",
            "tinfoil.dependencies",
        ),
        Id::BitBakeRecipeSources => backend_with_version_fallback(
            "BitBake recipe source metadata",
            "recipe_sources",
            "tinfoil.recipe_sources",
        ),
        Id::BitBakeRecipeMetadata => backend_with_version_fallback(
            "BitBake recipe metadata",
            "recipe_metadata",
            "tinfoil.recipe_metadata",
        ),
        Id::BitBakeLayerInventory => {
            backend_with_version_fallback("BitBake layer inventory", "layers", "tinfoil.layers")
        }
        Id::BitBakeLayerRelationships => backend_with_version_fallback(
            "BitBake layer relationships",
            "layer_relationships",
            "tinfoil.layer_relationships",
        ),
        Id::BitBakeBuild => {
            backend_with_version_fallback("BitBake build control", "build", "tinfoil.build")
        }
        Id::BitBakeCancellation => {
            backend_with_version_fallback("BitBake cancellation", "cancel", "tinfoil.cancel")
        }
        Id::BitBakeTaskList => {
            backend_with_version_fallback("BitBake task inventory", "tasks", "tinfoil.tasks")
        }
        Id::BitBakeForceTask => tool_command(
            "BitBake force-task execution",
            Tool::BitBake,
            None,
            &["-f", "-c"],
            "bitbake.force_task.argv",
        ),
        Id::BitBakeEnvironmentDump => tool_command(
            "BitBake environment dump",
            Tool::BitBake,
            None,
            &["-e"],
            "bitbake.environment_dump.argv",
        ),
        Id::BitBakeGraphGeneration => tool_command(
            "BitBake graph generation",
            Tool::BitBake,
            None,
            &["-g"],
            "bitbake.graph.argv",
        ),
        Id::BitBakeDependencyGraph => {
            let mut value = backend(
                "BitBake dependency graph",
                "dependency_graph",
                "tinfoil.dependency_graph",
            );
            value.6 = Some(FallbackImplementation {
                implementation: implementation("bitbake.graph.argv", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeGraphGeneration,
                },
            });
            value
        }
        Id::BitBakeGetVar => (
            "BitBake variable lookup",
            vec![Tool::BitBakeGetVar],
            vec![command(Tool::BitBakeGetVar, None, &["--value", "--recipe"])],
            Vec::new(),
            vec![
                executable(Tool::BitBakeGetVar),
                help(Tool::BitBakeGetVar, None),
                CapabilityProbeSpec::CommandOption {
                    tool: Tool::BitBakeGetVar,
                    subcommand: None,
                    option: "--value".into(),
                },
                CapabilityProbeSpec::CommandOption {
                    tool: Tool::BitBakeGetVar,
                    subcommand: None,
                    option: "--recipe".into(),
                },
            ],
            implementation("bitbake_getvar.argv", Kind::Command),
            Some(FallbackImplementation {
                implementation: implementation("bitbake.environment_lookup", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeEnvironmentDump,
                },
            }),
        ),
        Id::BitBakeVariableHistory => {
            let mut value = backend(
                "BitBake variable history",
                "variable_history",
                "tinfoil.variable_history",
            );
            value.6 = Some(FallbackImplementation {
                implementation: implementation("bitbake.environment_history", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeEnvironmentDump,
                },
            });
            value
        }
        Id::BitBakeDiffSigs => tool_command(
            "BitBake signature comparison",
            Tool::BitBakeDiffSigs,
            None,
            &["-c"],
            "bitbake_diffsigs.argv",
        ),
        Id::BitBakeDumpSig => tool_command(
            "BitBake signature dump",
            Tool::BitBakeDumpSig,
            None,
            &[],
            "bitbake_dumpsig.argv",
        ),
        Id::BitBakeServerSocket => backend_with_version_fallback(
            "BitBake server socket",
            "server_socket",
            "bitbake.server_socket",
        ),
        Id::BitBakeServerStatus => tool_command(
            "BitBake server status command",
            Tool::BitBake,
            None,
            &["--status-only"],
            "bitbake.server.status.argv",
        ),
        Id::BitBakeServerStart => tool_command(
            "BitBake server start command",
            Tool::BitBake,
            None,
            &["--server-only"],
            "bitbake.server.start.argv",
        ),
        Id::BitBakeServerStop => tool_command(
            "BitBake server stop command",
            Tool::BitBake,
            None,
            &["--kill-server"],
            "bitbake.server.stop.argv",
        ),
        Id::BitBakeNativeEvents => backend_with_version_fallback(
            "BitBake native events",
            "native_events",
            "tinfoil.native_events",
        ),
        _ => return None,
    })
}
