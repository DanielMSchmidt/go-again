use crate::FailedTest;
use std::io;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct TestResult {
    pub test: FailedTest,
    pub passed: bool,
    pub output: String,
}

/// Run a single test and return the result.
pub fn run_test(test: &FailedTest) -> io::Result<TestResult> {
    // Build the test filter pattern: ^TestName$
    // This ensures we match exactly this test (and subtests if any)
    let test_pattern = format!("^{}$", regex_escape(&test.name));

    let output = Command::new("go")
        .args([
            "test",
            &test.package,
            "-run",
            &test_pattern,
            "-v",
            "-count=1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{}{}", stdout, stderr);

    Ok(TestResult {
        test: test.clone(),
        passed: output.status.success(),
        output: combined_output,
    })
}

/// Run multiple tests and return results for each.
pub fn run_tests(tests: &[FailedTest]) -> Vec<TestResult> {
    tests
        .iter()
        .filter_map(|test| match run_test(test) {
            Ok(result) => Some(result),
            Err(e) => {
                eprintln!("Error running test {}: {}", test, e);
                None
            }
        })
        .collect()
}

/// Escape special regex characters in test names.
fn regex_escape(s: &str) -> String {
    let special_chars = [
        '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
    ];
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

/// Build the go test command for a test (useful for display/debugging).
pub fn build_test_command(test: &FailedTest) -> String {
    let test_pattern = format!("^{}$", regex_escape(&test.name));
    format!(
        "go test {} -run '{}' -v -count=1",
        test.package, test_pattern
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_escape_simple() {
        assert_eq!(regex_escape("TestFoo"), "TestFoo");
    }

    #[test]
    fn test_regex_escape_with_special_chars() {
        assert_eq!(regex_escape("TestFoo/case[1]"), "TestFoo/case\\[1\\]");
    }

    #[test]
    fn test_build_command() {
        let test = FailedTest {
            package: "./pkg/foo".to_string(),
            name: "TestBar".to_string(),
        };
        let cmd = build_test_command(&test);
        assert_eq!(cmd, "go test ./pkg/foo -run '^TestBar$' -v -count=1");
    }

    #[test]
    fn test_build_command_with_subtest() {
        let test = FailedTest {
            package: "./pkg".to_string(),
            name: "TestParent/subtest".to_string(),
        };
        let cmd = build_test_command(&test);
        assert_eq!(cmd, "go test ./pkg -run '^TestParent/subtest$' -v -count=1");
    }
}
