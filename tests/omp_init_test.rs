//! End-to-end coverage for the Pi/Oh My Pi extension lifecycle.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn run_rtk(cwd: &Path, agent_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .env("LC_ALL", "C")
        .env("HOME", cwd.join("home"))
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
    assert!(stderr(&install).contains("share the global extension path"));

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
    assert!(
        stdout(&dry_run).contains("[dry-run] would prompt before removing shared Pi/OMP extension")
    );
    assert!(stdout(&dry_run).contains("[dry-run] Nothing written."));

    let skipped = run_rtk(
        project.path(),
        &agent_dir,
        &[
            "init",
            "--agent",
            "omp",
            "--global",
            "--uninstall",
            "--no-patch",
        ],
    );
    assert!(
        !skipped.status.success(),
        "OMP uninstall skip unexpectedly succeeded: {}",
        stderr(&skipped)
    );
    assert!(stdout(&skipped).contains("Skipped removal of shared Pi/OMP extension"));
    assert!(stderr(&skipped).contains("was not removed"));
    assert!(agent_dir.join("extensions/rtk.ts").exists());

    let uninstall = run_rtk(
        project.path(),
        &agent_dir,
        &[
            "init",
            "--agent",
            "omp",
            "--global",
            "--uninstall",
            "--auto-patch",
        ],
    );
    assert!(
        uninstall.status.success(),
        "OMP uninstall failed: {}",
        stderr(&uninstall)
    );
    assert!(stdout(&uninstall).contains("Restart OMP to apply changes."));
    assert!(stderr(&uninstall).contains("share the global extension path"));
    assert!(!agent_dir.join("extensions/rtk.ts").exists());
}

#[test]
fn pi_dry_run_modified_extension_previews_confirmation_without_error() {
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
    assert!(stdout(&output).contains("[dry-run] would prompt before overwriting"));
    assert!(stdout(&output).contains("[dry-run] Nothing written."));
    assert_eq!(std::fs::read_to_string(&extension).unwrap(), original);

    let auto_dry_run = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--auto-patch", "--dry-run"],
    );
    assert!(
        auto_dry_run.status.success(),
        "Pi auto-patch dry-run failed: {}",
        stderr(&auto_dry_run)
    );
    assert!(stdout(&auto_dry_run).contains("[dry-run] would overwrite non-stock"));
    assert_eq!(std::fs::read_to_string(&extension).unwrap(), original);

    let auto = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--auto-patch"],
    );
    assert!(
        auto.status.success(),
        "Pi auto-patch failed: {}",
        stderr(&auto)
    );
    assert_eq!(
        std::fs::read_to_string(extension).unwrap(),
        include_str!("../hooks/pi/rtk.ts")
    );
}

#[test]
fn pi_relocated_global_without_omp_does_not_warn_or_prompt() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("pi-agent");

    let install = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--global"],
    );
    assert!(
        install.status.success(),
        "Pi global install failed: {}",
        stderr(&install)
    );
    assert!(!stderr(&install).contains("share the global extension path"));

    let uninstall = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--global", "--uninstall"],
    );
    assert!(
        uninstall.status.success(),
        "Pi global uninstall unexpectedly prompted or failed: {}",
        stderr(&uninstall)
    );
    assert!(!agent_dir.join("extensions/rtk.ts").exists());
}

#[test]
fn pi_modified_uninstall_dry_run_is_non_failing_preview() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("pi-agent");
    let extension_dir = project.path().join(".pi/extensions");
    std::fs::create_dir_all(&extension_dir).unwrap();
    let extension = extension_dir.join("rtk.ts");
    std::fs::write(
        &extension,
        format!(
            "{}\n// user modification\n",
            include_str!("../hooks/pi/rtk.ts")
        ),
    )
    .unwrap();

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "pi", "--uninstall", "--dry-run"],
    );

    assert!(
        output.status.success(),
        "Pi uninstall dry-run failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("[dry-run] would refuse to remove Pi extension"));
    assert!(stdout(&output).contains("[dry-run] Nothing written."));
    assert!(extension.exists());
}

#[test]
fn omp_modified_uninstall_dry_run_is_non_failing_preview() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = project.path().join("omp-agent");
    let extension_dir = project.path().join(".omp/extensions");
    std::fs::create_dir_all(&extension_dir).unwrap();
    let extension = extension_dir.join("rtk.ts");
    std::fs::write(
        &extension,
        format!(
            "{}\n// user modification\n",
            include_str!("../hooks/pi/rtk.ts")
        ),
    )
    .unwrap();

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "omp", "--uninstall", "--dry-run"],
    );

    assert!(
        output.status.success(),
        "OMP uninstall dry-run failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("[dry-run] would refuse to remove OMP extension"));
    assert!(stdout(&output).contains("[dry-run] Nothing written."));
    assert!(extension.exists());
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

#[test]
fn omp_show_reports_unreadable_extension_and_continues() {
    let project = TempDir::new().unwrap();
    let agent_dir = project.path().join("omp-agent");
    let extension_dir = agent_dir.join("extensions");
    std::fs::create_dir_all(&extension_dir).unwrap();
    std::fs::write(extension_dir.join("rtk.ts"), [0xff, 0xfe, 0xfd]).unwrap();

    let output = run_rtk(
        project.path(),
        &agent_dir,
        &["init", "--agent", "omp", "--global", "--show"],
    );

    assert!(
        output.status.success(),
        "OMP show failed for unreadable extension: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("Global extension:"));
    assert!(stdout(&output).contains("(unreadable)"));
    assert!(stdout(&output).contains("Project extension:"));
}
