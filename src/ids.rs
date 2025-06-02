use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            pub fn new(value: $inner) -> Self {
                Self(value)
            }

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
    };
}

// Define ID types for the application
define_id!(RepositoryId, i64);
define_id!(IssueId, i64);
define_id!(PullRequestId, i64);
define_id!(CommentId, i64);
define_id!(IssueNumber, i64);
define_id!(PullRequestNumber, i64);

// For sync status and cross references
define_id!(SyncStatusId, i64);
