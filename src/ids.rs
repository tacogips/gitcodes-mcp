use native_db::ToKey;
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Creates a new instance of the ID type.
            ///
            /// # Arguments
            ///
            /// * `value` - The underlying value for the ID
            pub fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Returns the underlying value of the ID.
            ///
            /// # Returns
            ///
            /// The inner value of the ID type
            pub fn value(&self) -> $inner {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl ToKey for $name {
            fn to_key(&self) -> native_db::Key {
                self.0.to_key()
            }

            fn key_names() -> Vec<String> {
                <$inner as ToKey>::key_names()
            }
        }
    };
}

// Define ID types for the application
define_id!(RepositoryId, i64);
define_id!(IssueId, i64);
define_id!(PullRequestId, i64);
define_id!(CommentId, i64);
define_id!(IssueNumber, i64);
define_id!(PullRequestNumber, i64);
define_id!(UserId, i64);

// For sync status and cross references
define_id!(SyncStatusId, i64);

// For GitHub Projects V2
// ProjectId uses String (GraphQL node ID) so we need a custom implementation without Copy
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Creates a new instance of the ProjectId.
    ///
    /// # Arguments
    ///
    /// * `value` - The underlying value for the ID
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns a reference to the underlying value of the ID.
    ///
    /// # Returns
    ///
    /// A string slice of the ID value
    pub fn value(&self) -> &str {
        &self.0
    }
    
    /// Consumes self and returns the underlying String.
    ///
    /// # Returns
    ///
    /// The inner String value
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ProjectId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl ToKey for ProjectId {
    fn to_key(&self) -> native_db::Key {
        self.0.to_key()
    }

    fn key_names() -> Vec<String> {
        <String as ToKey>::key_names()
    }
}

define_id!(ProjectNumber, i64);