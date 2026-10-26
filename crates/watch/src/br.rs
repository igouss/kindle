use std::path::Path;
use std::process::{Command, Output};

const READ_ONLY: [&str; 3] = ["--no-auto-flush", "--no-auto-import", "--allow-stale"];

pub fn run(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let output: Output = Command::new("br")
        .args(args)
        .args(READ_ONLY)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .map_err(|e: std::io::Error| format!("br {}: {e}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "br {} in {}: {}",
            args.join(" "),
            workspace.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
