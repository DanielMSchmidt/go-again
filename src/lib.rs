pub mod parser;
pub mod project;
pub mod runner;
pub mod storage;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedTest {
    pub package: String,
    pub name: String,
}

impl std::fmt::Display for FailedTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.package, self.name)
    }
}
