//! Doctor command.
use super::*;

#[cfg(unix)]
pub(crate) fn daemon_doctor_compatibility_report() -> DoctorCompatibilityReport {
    use yoctui_protocol::daemon::{ClientMessage, ServerMessage};
    match daemon_connection_with_snapshot() {
        Ok((mut connection, snapshot)) => {
            let report = doctor_compatibility_report(
                snapshot.compatibility.as_ref(),
                Some("Daemon snapshot has no compatibility authority."),
            );
            let _ = connection.send(&ClientMessage::Detach);
            let _ = connection.receive::<ServerMessage>();
            report
        }
        Err(error) => doctor_compatibility_report(
            None,
            Some(&format!("Yoctui daemon is unavailable: {error}")),
        ),
    }
}

#[cfg(not(unix))]
pub(crate) fn daemon_doctor_compatibility_report() -> DoctorCompatibilityReport {
    doctor_compatibility_report(
        None,
        Some("Daemon compatibility authority requires supported local IPC."),
    )
}

pub(crate) async fn doctor(build_dir: &Path, json: bool) -> Result<()> {
    let compatibility = daemon_doctor_compatibility_report();
    if json {
        println!("{}", serde_json::to_string_pretty(&compatibility)?);
        return Ok(());
    }
    let initialized = std::env::var_os("BUILDDIR").is_some();
    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
    let bitbake = tokio::process::Command::new("bitbake")
        .arg("--version")
        .output()
        .await;
    println!(
        "environment initialized: {}",
        if initialized {
            "yes"
        } else {
            "no — source oe-init-build-env"
        }
    );
    println!(
        "build directory: {} ({})",
        build_dir.display(),
        if build_dir.is_dir() {
            "usable"
        } else {
            "missing"
        }
    );
    match bitbake {
        Ok(o) => println!(
            "bitbake: {}",
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("available")
        ),
        Err(_) => {
            println!("bitbake: unavailable — source oe-init-build-env or add bitbake to PATH")
        }
    };
    match tokio::process::Command::new(&python)
        .args([
            "-c",
            "import bb; print(getattr(bb, '__version__', 'available'))",
        ])
        .output()
        .await
    {
        Ok(output) if output.status.success() => println!(
            "BitBake Python module: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        ),
        Ok(_) | Err(_) => println!(
            "BitBake Python module: unavailable — source oe-init-build-env before starting Yoctui"
        ),
    }
    for f in ["conf/local.conf", "conf/bblayers.conf"] {
        println!(
            "{}: {}",
            f,
            if build_dir.join(f).is_file() {
                "present"
            } else {
                "not found (may be normal outside a build dir)"
            }
        )
    }
    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
    match spawn_configured_bridge(&python, build_dir.to_path_buf(), None).await {
        Ok(mut bridge) => {
            let shutdown = bridge.shutdown().await;
            match shutdown {
                Ok(()) => println!("bridge protocol: ok (bounded handshake and shutdown)"),
                Err(error) => println!(
                    "bridge protocol: failed during shutdown ({error}) — check the active Python/BitBake environment"
                ),
            }
        }
        Err(error) => {
            println!("bridge startup: failed ({error}) — check YOCTUI_BRIDGE_PATH and PYTHON")
        }
    }
    println!("{}", render_doctor_compatibility(&compatibility));
    Ok(())
}
