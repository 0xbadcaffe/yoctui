use super::*;
use crate::VerifiedPtyEnvironment;
use std::{collections::BTreeMap, io::Write as _};

fn fixture() -> (PathBuf, PtyMenuconfigRouter, RecipeIdentity, RecipeIdentity) {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-pty-menuconfig-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("source")).unwrap();
    fs::create_dir_all(root.join("build")).unwrap();
    let bitbake = root.join("bitbake");
    let mut file = fs::File::create(&bitbake).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bitbake, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let contexts = PtyContextAuthority::new(
        "workspace".into(),
        root.join("source"),
        root.join("build"),
        VerifiedPtyEnvironment {
            identity: "build-env".into(),
            shell: fs::canonicalize("/bin/sh").unwrap(),
            environment: BTreeMap::new(),
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let kernel = RecipeIdentity {
        name: "virtual/kernel".into(),
        file: root.join("source/linux.bb"),
    };
    let uboot = RecipeIdentity {
        name: "u-boot".into(),
        file: root.join("source/u-boot.bb"),
    };
    let recipes = vec![
        PtyInteractiveRecipe {
            identity: kernel.clone(),
            tasks: BTreeSet::from(["menuconfig".into(), "devshell".into(), "nconfig".into()]),
        },
        PtyInteractiveRecipe {
            identity: uboot.clone(),
            tasks: BTreeSet::from(["menuconfig".into()]),
        },
    ];
    let router = PtyMenuconfigRouter::new(
        contexts,
        bitbake,
        recipes,
        Some(kernel.clone()),
        Some(uboot.clone()),
    )
    .unwrap();
    (root, router, kernel, uboot)
}

mod pty_menuconfig_previews_exact_recipe_kernel_uboot_and_devshell_argv;

mod pty_menuconfig_rejects_stale_recipe_and_unavailable_task;
