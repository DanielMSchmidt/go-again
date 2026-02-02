use crate::FailedTest;
use std::io::{BufRead, BufReader, Read};

/// Parse go test output and extract failed tests.
///
/// Handles patterns:
/// - `--- FAIL: TestName` - marks a failing test
/// - `panic: ...` followed by test info - marks a panicking test
/// - `FAIL\tpackage/path` - marks package with failures
/// - Ignores `FAIL\tpackage [build failed]` - compiler errors
pub fn parse_go_test_output<R: Read>(reader: R) -> Vec<FailedTest> {
    let reader = BufReader::new(reader);
    let mut failed_tests: Vec<FailedTest> = Vec::new();
    let mut current_package: Option<String> = None;
    let mut pending_test_names: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Track current package from "=== RUN" or package result lines
        if let Some(pkg) = extract_package_from_run_line(&line) {
            current_package = Some(pkg);
        }

        // Check for failed test: "--- FAIL: TestName"
        if let Some(test_name) = extract_failed_test_name(&line) {
            pending_test_names.push(test_name);
        }

        // Check for panic in test (also counts as failure)
        if line.contains("panic:") {
            // The test name should have been captured from a previous FAIL line
            // or will be in the stack trace - we handle this via the FAIL line
        }

        // Package result line: "FAIL\tpackage/path\t0.123s" or "FAIL\tpackage [build failed]"
        if let Some(pkg) = extract_package_from_fail_line(&line) {
            // Skip build failures
            if line.contains("[build failed]") {
                pending_test_names.clear();
                current_package = None;
                continue;
            }

            // Associate pending test names with this package
            for test_name in pending_test_names.drain(..) {
                failed_tests.push(FailedTest {
                    package: pkg.clone(),
                    name: test_name,
                });
            }
            current_package = Some(pkg);
        }

        // Also handle "ok" lines to reset state for next package
        if line.starts_with("ok \t") || line.starts_with("ok\t") {
            pending_test_names.clear();
        }
    }

    // Handle any remaining pending tests with last known package
    if let Some(pkg) = current_package {
        for test_name in pending_test_names {
            failed_tests.push(FailedTest {
                package: pkg.clone(),
                name: test_name,
            });
        }
    }

    failed_tests
}

fn extract_failed_test_name(line: &str) -> Option<String> {
    // Pattern: "--- FAIL: TestName (0.00s)"
    let line = line.trim();
    if line.starts_with("--- FAIL:") {
        let rest = line.strip_prefix("--- FAIL:")?.trim();
        // Test name is the first word
        let test_name = rest.split_whitespace().next()?;
        return Some(test_name.to_string());
    }
    None
}

fn extract_package_from_run_line(_line: &str) -> Option<String> {
    // Pattern: "=== RUN   TestName" doesn't give us package
    // But "?   \tpackage\t[no test files]" does
    // We primarily get package from FAIL/ok lines
    None
}

fn extract_package_from_fail_line(line: &str) -> Option<String> {
    // Pattern: "FAIL\tgithub.com/user/repo/pkg\t0.123s"
    // or: "FAIL\t./internal/logic\t0.005s"
    let line = line.trim();
    if !line.starts_with("FAIL\t") && !line.starts_with("FAIL ") {
        return None;
    }

    let rest = line.strip_prefix("FAIL")?.trim();
    // Package is the first tab-separated field
    let package = rest.split('\t').next()?.trim();
    if package.is_empty() {
        return None;
    }
    Some(package.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_failure() {
        let input = r#"=== RUN   TestCalculateSum
--- FAIL: TestCalculateSum (0.00s)
    logic_test.go:15: expected 5, got 4
FAIL	./internal/logic	0.005s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].package, "./internal/logic");
        assert_eq!(result[0].name, "TestCalculateSum");
    }

    #[test]
    fn test_multiple_failures_same_package() {
        let input = r#"=== RUN   TestAdd
--- FAIL: TestAdd (0.00s)
=== RUN   TestSubtract
--- FAIL: TestSubtract (0.00s)
FAIL	./math	0.003s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].package, "./math");
        assert_eq!(result[0].name, "TestAdd");
        assert_eq!(result[1].package, "./math");
        assert_eq!(result[1].name, "TestSubtract");
    }

    #[test]
    fn test_failures_across_packages() {
        let input = r#"=== RUN   TestFoo
--- FAIL: TestFoo (0.00s)
FAIL	./pkg/foo	0.002s
=== RUN   TestBar
--- FAIL: TestBar (0.00s)
FAIL	./pkg/bar	0.003s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].package, "./pkg/foo");
        assert_eq!(result[0].name, "TestFoo");
        assert_eq!(result[1].package, "./pkg/bar");
        assert_eq!(result[1].name, "TestBar");
    }

    #[test]
    fn test_build_failure_ignored() {
        let input = r#"# ./broken
./broken/main.go:5:1: syntax error
FAIL	./broken [build failed]
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert!(result.is_empty());
    }

    #[test]
    fn test_panic_captured() {
        let input = r#"=== RUN   TestPanic
--- FAIL: TestPanic (0.00s)
panic: runtime error: index out of range [recovered]
FAIL	./panicky	0.004s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].package, "./panicky");
        assert_eq!(result[0].name, "TestPanic");
    }

    #[test]
    fn test_all_passing() {
        let input = r#"=== RUN   TestGood
--- PASS: TestGood (0.00s)
ok  	./good	0.002s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert!(result.is_empty());
    }

    #[test]
    fn test_mixed_pass_fail() {
        let input = r#"=== RUN   TestPass
--- PASS: TestPass (0.00s)
=== RUN   TestFail
--- FAIL: TestFail (0.00s)
FAIL	./mixed	0.003s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "TestFail");
    }

    #[test]
    fn test_subtest_failure() {
        let input = r#"=== RUN   TestParent
=== RUN   TestParent/subtest1
--- FAIL: TestParent/subtest1 (0.00s)
--- FAIL: TestParent (0.00s)
FAIL	./subtests	0.002s
"#;
        let result = parse_go_test_output(input.as_bytes());
        // We capture both the subtest and parent
        assert!(result.iter().any(|t| t.name == "TestParent/subtest1"));
    }

    #[test]
    fn test_full_package_path() {
        let input = r#"=== RUN   TestHandler
--- FAIL: TestHandler (0.00s)
FAIL	github.com/user/repo/api/handler	0.010s
"#;
        let result = parse_go_test_output(input.as_bytes());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].package, "github.com/user/repo/api/handler");
    }
}
