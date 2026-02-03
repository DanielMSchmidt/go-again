use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn go_again_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("go-again").unwrap()
}

fn go_again_cmd_with_state_dir(dir: &TempDir) -> Command {
    let mut cmd = go_again_cmd();
    cmd.env("GO_AGAIN_STATE_DIR", dir.path());
    cmd
}

#[test]
fn test_help() {
    go_again_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Remember and re-run failing Go tests",
        ));
}

#[test]
fn test_remember_help() {
    go_again_cmd()
        .args(["remember", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("go test output"));
}

#[test]
fn test_run_help() {
    go_again_cmd()
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--update"));
}

#[test]
fn test_clear_command() {
    go_again_cmd()
        .arg("clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared remembered tests"));
}

#[test]
fn test_remember_with_passing_tests() {
    let input = r#"=== RUN   TestGood
--- PASS: TestGood (0.00s)
ok  	./good	0.002s
"#;

    go_again_cmd()
        .arg("remember")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("All tests passed"));
}

#[test]
fn test_remember_with_failing_tests() {
    let input = r#"=== RUN   TestBad
--- FAIL: TestBad (0.00s)
    bad_test.go:10: expected true
FAIL	./bad	0.003s
"#;

    go_again_cmd()
        .arg("remember")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Remembered 1 failing test"))
        .stdout(predicate::str::contains("./bad TestBad"));
}

#[test]
fn test_remember_multiple_failures() {
    let input = r#"=== RUN   TestOne
--- FAIL: TestOne (0.00s)
=== RUN   TestTwo
--- FAIL: TestTwo (0.00s)
FAIL	./multi	0.005s
"#;

    go_again_cmd()
        .arg("remember")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Remembered 2 failing test"));
}

#[test]
fn test_remember_ignores_build_failures() {
    let input = r#"# ./broken
./broken/main.go:5:1: syntax error
FAIL	./broken [build failed]
"#;

    go_again_cmd()
        .arg("remember")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("All tests passed"));
}

// Integration test for the full workflow - tests the complete cycle
// Uses isolated state directory for test isolation
#[test]
fn test_full_workflow() {
    let state_dir = TempDir::new().unwrap();

    // Step 1: Verify list shows no tests (fresh state)
    go_again_cmd_with_state_dir(&state_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No failing tests"));

    // Step 2: Verify run shows no tests
    go_again_cmd_with_state_dir(&state_dir)
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("No failing tests"));

    // Step 3: Remember a failing test
    let input = r#"=== RUN   TestWorkflow
--- FAIL: TestWorkflow (0.00s)
FAIL	./workflow	0.001s
"#;

    go_again_cmd_with_state_dir(&state_dir)
        .arg("remember")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Remembered 1 failing test"));

    // Step 4: List should show the test
    go_again_cmd_with_state_dir(&state_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("./workflow TestWorkflow"));

    // Step 5: Clear
    go_again_cmd_with_state_dir(&state_dir)
        .arg("clear")
        .assert()
        .success();

    // Step 6: List should show no tests
    go_again_cmd_with_state_dir(&state_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No failing tests"));
}
