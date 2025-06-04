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