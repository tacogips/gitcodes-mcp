use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Issue,
    PullRequest,
}

impl ItemType {
    /// Returns the string representation for unknown item types.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Open,
    Closed,
}

impl IssueState {
    /// Returns the string representation for unknown issue states.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

impl PullRequestState {
    /// Returns the string representation for unknown pull request states.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Issues,
    PullRequests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SyncStatusType {
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Github,
}

/// A strongly-typed repository name (format: owner/repo)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryName(String);

impl RepositoryName {
    /// Creates a new RepositoryName from a string
    /// 
    /// # Arguments
    /// 
    /// * `name` - The repository name in "owner/repo" format
    /// 
    /// # Returns
    /// 
    /// Returns Ok(RepositoryName) if the format is valid, otherwise returns an error
    pub fn new(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        if name.split('/').count() == 2 && !name.contains(' ') && !name.is_empty() {
            Ok(RepositoryName(name))
        } else {
            Err(format!("Invalid repository name format: '{}'. Expected format: 'owner/repo'", name))
        }
    }
    
    /// Returns the repository name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Consumes self and returns the inner String
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for RepositoryName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for RepositoryName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RepositoryName::new(s)
    }
}
