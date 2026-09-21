use super::*;

fn digest(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn key() -> CapabilityCacheKey {
    CapabilityCacheKey {
        environment: YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                "/workspace/build".into(),
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                "2.8.1".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        },
        workspace_identity: "/workspace/poky@0123456789abcdef".into(),
        initialized_environment_digest: digest('a'),
        layer_configuration_digest: digest('b'),
        build_configuration_digest: digest('c'),
        daemon_workspace_identity: "daemon-workspace-one".into(),
    }
}

mod compatibility_cache_key_preserves_every_invalidation_dimension;

mod compatibility_cache_key_rejects_weak_digest_or_invalid_identity;
