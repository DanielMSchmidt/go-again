use crate::FailedTest;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

pub type StateMap = HashMap<String, Vec<FailedTest>>;

fn get_state_file_path() -> Option<PathBuf> {
    // Allow overriding state directory for testing
    if let Ok(dir) = std::env::var("GO_AGAIN_STATE_DIR") {
        return Some(PathBuf::from(dir).join("state.json"));
    }
    let home = dirs::home_dir()?;
    Some(home.join(".go-again").join("state.json"))
}

pub fn load() -> io::Result<StateMap> {
    load_from_path(get_state_file_path())
}

pub fn load_from_path(path: Option<PathBuf>) -> io::Result<StateMap> {
    let path = match path {
        Some(p) => p,
        None => return Ok(HashMap::new()),
    };

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&path)?;
    if content.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let state: StateMap = serde_json::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse state file: {}", e),
        )
    })?;
    Ok(state)
}

pub fn save(state: &StateMap) -> io::Result<()> {
    save_to_path(state, get_state_file_path())
}

pub fn save_to_path(state: &StateMap, path: Option<PathBuf>) -> io::Result<()> {
    let path = match path {
        Some(p) => p,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn get_for_project(key: &str) -> io::Result<Vec<FailedTest>> {
    let state = load()?;
    Ok(state.get(key).cloned().unwrap_or_default())
}

pub fn set_for_project(key: &str, tests: Vec<FailedTest>) -> io::Result<()> {
    let mut state = load()?;
    if tests.is_empty() {
        state.remove(key);
    } else {
        state.insert(key.to_string(), tests);
    }
    save(&state)
}

pub fn clear_for_project(key: &str) -> io::Result<()> {
    set_for_project(key, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_path(dir: &TempDir) -> Option<PathBuf> {
        Some(dir.path().join("state.json"))
    }

    #[test]
    fn test_load_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let result = load_from_path(test_path(&dir)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        let mut state = StateMap::new();
        state.insert(
            "/project:main".to_string(),
            vec![FailedTest {
                package: "./pkg".to_string(),
                name: "TestFoo".to_string(),
            }],
        );

        save_to_path(&state, path.clone()).unwrap();
        let loaded = load_from_path(path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("/project:main").unwrap().len(), 1);
        assert_eq!(loaded.get("/project:main").unwrap()[0].name, "TestFoo");
    }

    #[test]
    fn test_update_existing_state() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        // Initial save
        let mut state = StateMap::new();
        state.insert(
            "/project:main".to_string(),
            vec![FailedTest {
                package: "./pkg".to_string(),
                name: "TestFoo".to_string(),
            }],
        );
        save_to_path(&state, path.clone()).unwrap();

        // Update
        let mut state = load_from_path(path.clone()).unwrap();
        state.insert(
            "/project:feature".to_string(),
            vec![FailedTest {
                package: "./other".to_string(),
                name: "TestBar".to_string(),
            }],
        );
        save_to_path(&state, path.clone()).unwrap();

        // Verify both entries exist
        let loaded = load_from_path(path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains_key("/project:main"));
        assert!(loaded.contains_key("/project:feature"));
    }

    #[test]
    fn test_empty_tests_removes_key() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        // Initial save with data
        let mut state = StateMap::new();
        state.insert(
            "/project:main".to_string(),
            vec![FailedTest {
                package: "./pkg".to_string(),
                name: "TestFoo".to_string(),
            }],
        );
        save_to_path(&state, path.clone()).unwrap();

        // Clear by setting empty vec
        let mut state = load_from_path(path.clone()).unwrap();
        state.remove("/project:main");
        save_to_path(&state, path.clone()).unwrap();

        let loaded = load_from_path(path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("state.json");

        let state = StateMap::new();
        save_to_path(&state, Some(path.clone())).unwrap();

        assert!(path.exists());
    }
}
