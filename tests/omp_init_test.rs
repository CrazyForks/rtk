//! End-to-end coverage for the Pi/Oh My Pi extension lifecycle.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn run_rtk(cwd: &Path, agent_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .env("LC_ALL", "C")
        .env("PI_CODING_AGENT_DIR", agent_dir)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn rtk")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn omp_dry_run_missing_includes_footer() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("omp-agent");

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &[
            "init",
            "--agent",
            "omp",
            "--global",
            "--uninstall",
            "--dry-run",
        ],
    );

    assert!(
        output.status.success(),
        "OMP dry-run failed: {}",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("[dry-run] Nothing written."),
        "missing dry-run footer: {}",
        stdout(&output)
    );
}

#[test]
fn omp_dry_run_stock_includes_footer_and_real_uninstall_mentions_restart() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("omp-agent");

    let install = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "omp", "--global"],
    );
    assert!(
        install.status.success(),
        "OMP install failed: {}",
        stderr(&install)
    );

    let dry_run = run_rtk(
        project.path(),
        &agent_dir,
        &[
            "init",
            "--agent",
            "omp",
            "--global",
            "--uninstall",
            "--dry-run",
        ],
    );
    assert!(
        dry_run.status.success(),
        "OMP uninstall dry-run failed: {}",
        stderr(&dry_run)
    );
    assert!(stdout(&dry_run).contains("[dry-run] would remove OMP extension"));
    assert!(stdout(&dry_run).contains("[dry-run] Nothing written."));

    let uninstall = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "omp", "--global", "--uninstall"],
    );
    assert!(
        uninstall.status.success(),
        "OMP uninstall failed: {}",
        stderr(&uninstall)
    );
    assert!(stdout(&uninstall).contains("Restart OMP to apply changes."));
    assert!(stderr(&uninstall).contains("share the global extension path"));
}

#[test]
fn pi_dry_run_modified_extension_reports_refusal_without_error() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("pi-agent");
    let extension_dir = project.path().join(".pi/extensions");
    std::fs::create_dir_all(&extension_dir).unwrap();
    let extension = extension_dir.join("rtk.ts");
    let original = "// user-modified extension\nexport default () => {}\n";
    std::fs::write(&extension, original).unwrap();

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--dry-run"],
    );

    assert!(
        output.status.success(),
        "Pi dry-run failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("[dry-run] would refuse to overwrite"));
    assert!(stdout(&output).contains("[dry-run] Nothing written."));
    assert_eq!(std::fs::read_to_string(extension).unwrap(), original);
}

#[test]
fn omp_show_modified_extension_explains_install_refusal() {
    let project = TempDir::new().unwrap();
    let agent_dir = project.path().join("omp-agent");
    let extension_dir = agent_dir.join("extensions");
    std::fs::create_dir_all(&extension_dir).unwrap();
    std::fs::write(
        extension_dir.join("rtk.ts"),
        "// user-modified extension\nexport default () => {}\n",
    )
    .unwrap();

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "omp", "--global", "--show"],
    );

    assert!(
        output.status.success(),
        "OMP show failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("rtk init will refuse to overwrite"));
}
