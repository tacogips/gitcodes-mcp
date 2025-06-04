// Example implementation of User model for GitDB
// This file demonstrates how to implement unique participant persistence

use chrono::{DateTime, Utc};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use crate::ids::UserId;

/// Represents a GitHub user/participant in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 8, version = 1)]
#[native_db]
pub struct User {
    /// GitHub user ID - primary key, guaranteed unique
    #[primary_key]
    pub id: UserId,
    
    /// GitHub username - secondary key with unique constraint
    #[secondary_key(unique)]
    pub login: String,
    
    /// User's avatar URL from GitHub
    pub avatar_url: Option<String>,
    
    /// User's profile URL on GitHub
    pub html_url: Option<String>,
    
    /// Type of user: "User" or "Bot"
    pub user_type: String,
    
    /// Whether user is a GitHub site administrator
    pub site_admin: bool,
    
    /// When we first saw this user in our system
    pub first_seen_at: DateTime<Utc>,
    
    /// Last time we updated this user's information
    pub last_updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new User instance from GitHub API data
    pub fn from_github_user(
        id: i64,
        login: String,
        avatar_url: Option<String>,
        html_url: Option<String>,
        user_type: String,
        site_admin: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(id),
            login,
            avatar_url,
            html_url,
            user_type,
            site_admin,
            first_seen_at: now,
            last_updated_at: now,
        }
    }
    
    /// Updates user information from fresh GitHub data
    pub fn update_from_github(
        &mut self,
        avatar_url: Option<String>,
        html_url: Option<String>,
        user_type: String,
        site_admin: bool,
    ) {
        self.avatar_url = avatar_url;
        self.html_url = html_url;
        self.user_type = user_type;
        self.site_admin = site_admin;
        self.last_updated_at = Utc::now();
    }
}

/// Example database operations for User model
impl super::GitDatabase {
    /// Gets or creates a user from GitHub data
    pub async fn get_or_create_user(
        &self,
        github_id: i64,
        login: &str,
        avatar_url: Option<String>,
        html_url: Option<String>,
        user_type: &str,
        site_admin: bool,
    ) -> anyhow::Result<User> {
        let user_id = UserId::new(github_id);
        
        // First try to get by primary key (ID)
        let rw = self.db.rw_transaction()?;
        
        match rw.get::<User>().primary(&user_id)? {
            Some(mut existing_user) => {
                // User exists, update if login changed
                if existing_user.login != login {
                    // Handle username change
                    existing_user.login = login.to_string();
                }
                existing_user.update_from_github(
                    avatar_url,
                    html_url,
                    user_type.to_string(),
                    site_admin,
                );
                rw.update(existing_user.clone())?;
                rw.commit()?;
                Ok(existing_user)
            }
            None => {
                // Create new user
                let new_user = User::from_github_user(
                    github_id,
                    login.to_string(),
                    avatar_url,
                    html_url,
                    user_type.to_string(),
                    site_admin,
                );
                rw.insert(new_user.clone())?;
                rw.commit()?;
                Ok(new_user)
            }
        }
    }
    
    /// Gets a user by their GitHub login (username)
    pub async fn get_user_by_login(&self, login: &str) -> anyhow::Result<Option<User>> {
        let r = self.db.r_transaction()?;
        Ok(r.get::<User>()
            .secondary(UserKey::login, &login)?
            .into_iter()
            .next())
    }
    
    /// Gets multiple users by their IDs
    pub async fn get_users_by_ids(&self, user_ids: &[UserId]) -> anyhow::Result<Vec<User>> {
        let r = self.db.r_transaction()?;
        let mut users = Vec::new();
        
        for user_id in user_ids {
            if let Some(user) = r.get::<User>().primary(user_id)? {
                users.push(user);
            }
        }
        
        Ok(users)
    }
    
    /// Gets all unique participants (authors, assignees, commenters) for a repository
    pub async fn get_repository_participants(
        &self,
        repository_id: &crate::ids::RepositoryId,
    ) -> anyhow::Result<Vec<User>> {
        use std::collections::HashSet;
        
        let r = self.db.r_transaction()?;
        let mut user_ids = HashSet::new();
        
        // Collect from issues
        let issues = self.list_issues_by_repository(repository_id).await?;
        for issue in issues {
            // Note: This would need to be updated once we add author_id field
            // For now, we'd need to look up users by login
            if let Some(user) = self.get_user_by_login(&issue.author).await? {
                user_ids.insert(user.id);
            }
            
            for assignee in &issue.assignees {
                if let Some(user) = self.get_user_by_login(assignee).await? {
                    user_ids.insert(user.id);
                }
            }
        }
        
        // Collect from pull requests
        let prs = self.list_pull_requests_by_repository(repository_id).await?;
        for pr in prs {
            if let Some(user) = self.get_user_by_login(&pr.author).await? {
                user_ids.insert(user.id);
            }
            
            for assignee in &pr.assignees {
                if let Some(user) = self.get_user_by_login(assignee).await? {
                    user_ids.insert(user.id);
                }
            }
        }
        
        // Convert set to vector and fetch full user objects
        let user_ids: Vec<UserId> = user_ids.into_iter().collect();
        self.get_users_by_ids(&user_ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User::from_github_user(
            12345,
            "octocat".to_string(),
            Some("https://github.com/octocat.png".to_string()),
            Some("https://github.com/octocat".to_string()),
            "User".to_string(),
            false,
        );
        
        assert_eq!(user.id.value(), 12345);
        assert_eq!(user.login, "octocat");
        assert_eq!(user.user_type, "User");
        assert!(!user.site_admin);
    }
    
    #[test]
    fn test_user_update() {
        let mut user = User::from_github_user(
            12345,
            "octocat".to_string(),
            None,
            None,
            "User".to_string(),
            false,
        );
        
        let original_created = user.first_seen_at;
        
        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        user.update_from_github(
            Some("https://github.com/octocat-new.png".to_string()),
            Some("https://github.com/octocat".to_string()),
            "User".to_string(),
            true,
        );
        
        assert_eq!(user.first_seen_at, original_created);
        assert!(user.last_updated_at > original_created);
        assert!(user.site_admin);
        assert!(user.avatar_url.is_some());
    }
}