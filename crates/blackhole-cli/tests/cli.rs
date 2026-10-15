//! Process-level CLI tests.
//!
//! These run the binary that cargo built, so they verify what a user or a
//! script actually observes: the exit status, the separation of results
//! (stdout) from diagnostics (stderr), and the byte-reproducibility of the
//! output. Unit tests can check none of those, because they call functions
//! inside one process.

use std::io::Write;
use std::process::{Command, Stdio};

/// The binary under test; cargo sets this variable for integration tests.
const BIN: &str = env!("CARGO_BIN_EXE_blackhole");

/// The observable result of one process run.
struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Runs the binary with `args`, optionally feeding `stdin` as scripted input.
///
/// Named `run_cli` rather than `run` so that a test can bind the result to a
/// local variable called `run` without shadowing the function.
fn run_cli(args: &[&str], stdin: Option<&str>) -> Run {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary must be spawnable");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("scripted input must be writable");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("the binary must exit");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[test]
fn known_object_writes_results_to_stdout_and_exits_zero() {
    let run = run_cli(&["object", "Earth"], None);
    assert_eq!(run.status, 0);
    assert!(
        run.stdout.contains("Schwarzschild radius r_s"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains("Important limitations"),
        "{}",
        run.stdout
    );
    assert!(run.stderr.is_empty(), "stderr: {}", run.stderr);
}

#[test]
fn values_are_checked_against_documented_reference_data() {
    // NASA NSSDCA Earth Fact Sheet mass and volumetric mean radius.
    // r_s = 2GM/c^2 = 8.870103e-3 m, computed independently in
    // docs/reports/m1_evidence.py (check C1).
    let earth = run_cli(&["object", "Earth"], None);
    assert!(earth.stdout.contains("5.972200e24 kg"), "{}", earth.stdout);
    assert!(earth.stdout.contains("8.870103e-3 m"), "{}", earth.stdout);

    // The Sun's Schwarzschild radius is about 2.95 km for 1.9884e30 kg.
    let sun = run_cli(&["object", "Sun"], None);
    assert!(sun.stdout.contains("2.953"), "{}", sun.stdout);
}

#[test]
fn a_negative_value_is_a_physics_error_reported_on_stderr() {
    let run = run_cli(&["calc", "--mass", "-5", "--radius", "1"], None);
    assert_eq!(
        run.status, 1,
        "stdout: {} stderr: {}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr
            .contains("mass must be greater than zero, got -5"),
        "stderr: {}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "stdout: {}", run.stdout);
}

#[test]
fn usage_errors_exit_two_and_name_the_problem() {
    let none = run_cli(&["calc"], None);
    assert_eq!(none.status, 2);
    assert!(
        none.stderr
            .contains("exactly two of --mass, --radius and --density"),
        "{}",
        none.stderr
    );

    let bad_unit = run_cli(
        &[
            "calc",
            "--mass",
            "1",
            "--mass-unit",
            "stone",
            "--radius",
            "1",
        ],
        None,
    );
    assert_eq!(bad_unit.status, 2);
    assert!(
        bad_unit.stderr.contains("unknown mass unit `stone`"),
        "{}",
        bad_unit.stderr
    );

    let bad_object = run_cli(&["object", "Vulcan"], None);
    assert_eq!(bad_object.status, 2);
    assert!(
        bad_object
            .stderr
            .contains("no catalogued object matches `Vulcan`"),
        "{}",
        bad_object.stderr
    );
    assert!(
        bad_object.stderr.contains("available:"),
        "{}",
        bad_object.stderr
    );

    let bad_material = run_cli(&["calc", "--radius", "1", "--density", "unobtainium"], None);
    assert_eq!(bad_material.status, 2);
    assert!(
        bad_material.stderr.contains("known materials:"),
        "{}",
        bad_material.stderr
    );
}

#[test]
fn identical_arguments_produce_byte_identical_output() {
    let first = run_cli(&["object", "Jupiter"], None);
    let second = run_cli(&["object", "Jupiter"], None);
    assert_eq!(first.status, 0);
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn the_interactive_dialogue_runs_to_completion() {
    let run = run_cli(&[], Some("3\nEarth\nn\n"));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    assert!(run.stdout.contains("Schwarzschild radius r_s"));
    assert!(run.stdout.contains("reference data"));
}

#[test]
fn closed_stdin_ends_the_interactive_session_cleanly() {
    let run = run_cli(&[], Some(""));
    assert_eq!(run.status, 0);
    assert!(run.stdout.contains("Choose input mode"));
}

#[test]
fn interactive_and_non_interactive_paths_agree_on_the_physics() {
    let batch = run_cli(
        &[
            "calc",
            "--radius",
            "1",
            "--radius-unit",
            "km",
            "--density",
            "water",
            "--label",
            "sphere",
        ],
        None,
    );
    let dialogue = run_cli(&[], Some("1\n1 km\nwater\nsphere\nq\n"));

    // Compare only the value rows, since the interactive run also prints its
    // prompts and the menu.
    let value_rows = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("Mass")
                    || trimmed.starts_with("Schwarzschild radius")
                    || trimmed.starts_with("Compactness ratio")
                    || trimmed.starts_with("Critical radius")
            })
            .map(|line| line.trim().to_string())
            .collect()
    };

    let batch_rows = value_rows(&batch.stdout);
    assert_eq!(batch_rows.len(), 4, "{}", batch.stdout);
    assert_eq!(batch_rows, value_rows(&dialogue.stdout));
}

#[test]
fn help_documents_the_model_and_the_exit_statuses_stay_distinct() {
    let help = run_cli(&["--help"], None);
    assert_eq!(help.status, 0);
    assert!(help.stdout.contains("Schwarzschild radius"));
    assert!(
        help.stdout
            .contains("does not predict whether a horizon forms")
    );

    assert_ne!(
        run_cli(&["object", "Earth"], None).status,
        run_cli(&["calc"], None).status
    );
}

#[test]
fn an_object_can_be_selected_by_index_in_both_paths() {
    // The interactive mode-3 prompt accepts a 1-based index; the subcommand must
    // accept the same token, or the two front ends disagree about valid input.
    let by_index = run_cli(&["object", "2"], None);
    assert_eq!(by_index.status, 0, "stderr: {}", by_index.stderr);
    assert!(by_index.stdout.contains("Earth"), "{}", by_index.stdout);

    let by_name = run_cli(&["object", "Earth"], None);
    assert_eq!(by_index.stdout, by_name.stdout);

    // Out of range, and a zero index, are usage errors.
    assert_eq!(run_cli(&["object", "0"], None).status, 2);
    assert_eq!(run_cli(&["object", "99"], None).status, 2);
}

#[test]
fn a_closed_stdout_pipe_does_not_panic() {
    // `println!` aborts with EPIPE when the reader goes away, which would make
    // this exit 101 — neither a usage error nor a physical failure. The label is
    // padded so that the report exceeds the pipe buffer, otherwise the child
    // finishes writing before the reader closes and the case proves nothing.
    let label = "x".repeat(70_000);
    let mut child = Command::new(BIN)
        .args([
            "calc",
            "--radius",
            "1",
            "--density",
            "1000",
            "--label",
            &label,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary must be spawnable");
    {
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let mut first = [0_u8; 16];
        let _ = std::io::Read::read(&mut stdout, &mut first);
        drop(stdout); // closing the read end makes further writes fail with EPIPE
    }
    let output = child.wait_with_output().expect("the binary must exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "stderr: {stderr}");
    assert_ne!(
        output.status.code(),
        Some(101),
        "a broken pipe must not be reported as a panic"
    );
}

#[test]
fn invalid_utf8_on_stdin_does_not_abort_the_dialogue() {
    let mut child = Command::new(BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary must be spawnable");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(b"\xff\xfe\n3\nEarth\nn\n")
        .expect("scripted input must be writable");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("the binary must exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "stdout: {stdout}");
    assert!(
        !stdout.contains("not valid UTF-8"),
        "the session aborted on the decode error: {stdout}"
    );
    assert!(
        stdout.contains("Schwarzschild radius r_s"),
        "the dialogue did not reach a result: {stdout}"
    );
}

#[test]
fn a_negative_density_is_a_physics_error_not_an_unknown_argument() {
    for args in [
        vec!["calc", "--radius", "1", "--density", "-5"],
        vec!["calc", "--radius", "1", "--density=-5"],
    ] {
        let run = run_cli(&args, None);
        assert_eq!(run.status, 1, "args {args:?}: {}", run.stderr);
        assert!(
            run.stderr
                .contains("density must be greater than zero, got -5"),
            "args {args:?}: {}",
            run.stderr
        );
    }
}

#[test]
fn a_missing_value_is_reported_as_such_and_not_as_a_stray_argument() {
    // `allow_negative_numbers` must not swallow the following flag: with
    // `allow_hyphen_values` this reported "unexpected argument '1' found".
    let run = run_cli(&["calc", "--mass", "--radius", "1"], None);
    assert_eq!(run.status, 2);
    assert!(
        run.stderr
            .contains("a value is required for '--mass <VALUE>'")
            || run.stderr.contains("requires a value"),
        "stderr: {}",
        run.stderr
    );
}
