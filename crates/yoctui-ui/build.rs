use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=YOCTUI_BUILD_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    let sha = std::env::var("YOCTUI_BUILD_SHA").ok().or_else(|| {
        Command::new("git")
            .args(["rev-parse", "--short=12", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|sha| sha.trim().to_owned())
            .filter(|sha| !sha.is_empty())
    });
    println!(
        "cargo:rustc-env=YOCTUI_BUILD_SHA={}",
        sha.as_deref().unwrap_or("unknown")
    );
}
