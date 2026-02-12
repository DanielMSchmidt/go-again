use std::io;
use std::process::Command;

/// Get the storage key for the current project.
/// Format: "{git_root}:{branch}"
pub fn get_project_key() -> io::Result<String> {
    let root = get_git_root()?;
    let branch = get_git_branch()?;
    Ok(format!("{}:{}", root, branch))
}

/// Get the git repository root directory.
pub fn get_git_root() -> io::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Not in a git repository",
        ));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(root)
}

/// Get the current git branch name.
fn get_git_branch() -> io::Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other("Failed to get git branch"));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Handle detached HEAD state
    if branch.is_empty() {
        // Try to get commit hash instead
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()?;

        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(format!("detached-{}", hash));
        }
        return Ok("detached".to_string());
    }

    Ok(branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_format() {
        // This test will only pass when run in a git repo
        if let Ok(key) = get_project_key() {
            assert!(key.contains(':'), "Key should contain colon separator");
            let parts: Vec<&str> = key.split(':').collect();
            assert_eq!(parts.len(), 2, "Key should have exactly two parts");
            assert!(!parts[0].is_empty(), "Root should not be empty");
            assert!(!parts[1].is_empty(), "Branch should not be empty");
        }
    }

    #[test]
    fn test_git_root_is_absolute_path() {
        if let Ok(root) = get_git_root() {
            assert!(
                root.starts_with('/') || root.chars().nth(1) == Some(':'),
                "Root should be an absolute path"
            );
        }
    }
}
