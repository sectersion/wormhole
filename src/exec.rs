use anyhow::Context as _;

/// Run a command, capture output. Honors dry_run via Ctx flag passed by caller.
pub fn cmd(prog: &str, args: &[&str]) -> anyhow::Result<String> {
    let out = std::process::Command::new(prog)
        .args(args)
        .output()
        .with_context(|| format!("spawn {prog}"))?;
    if !out.status.success() {
        anyhow::bail!("{} failed: {}", prog, String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        // best-effort: try opening a privileged op
        cmd("net", &["session"]).is_ok()
    }
    #[cfg(not(windows))]
    {
        unsafe { libc_euid() == 0 }
    }
}

#[cfg(not(windows))]
fn libc_euid() -> u32 {
    // avoid libc dep: read UID via id -u
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(9999)
}
