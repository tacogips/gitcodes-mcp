use strum::{AsRefStr, Display, EnumString};

#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct SearchParams {
    pub query: String,
}

impl SearchParams {
    pub fn construct_search_url(&self) -> String {}
}
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct GrepParams {
    pub exclude_dirs: Option<Vec<String>>,
}

#[derive(
    Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum SortOption {
    Updated,
}

impl SortOption {
    /// Converts the sort option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for SortOption {
    fn default() -> Self {
        SortOption::Relevance
    }
}

#[strum(serialize_all = "lowercase")]
pub enum OrderOption {
    Ascending,
    Descending,
}

impl OrderOption {
    /// Converts the order option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for OrderOption {
    /// Returns the default order option (Descending)
    fn default() -> Self {
        OrderOption::Descending
    }
}
