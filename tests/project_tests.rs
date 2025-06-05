use anyhow::Result;
use chrono::Utc;
use gitdb::ids::{IssueId, ProjectId, ProjectNumber, PullRequestId, RepositoryId};
use gitdb::storage::database::GitDatabase;
use gitdb::storage::models::{Issue, Project, ProjectItem, PullRequest, Repository};
use gitdb::types::{IssueState, ItemType, PullRequestState};

#[tokio::test]
async fn test_project_operations() -> Result<()> {
    let db = GitDatabase::new().await?;
    
    // Create a test repository
    let repo = Repository {
        id: RepositoryId::new(1),
        owner: "test-owner".to_string(),
        name: "test-repo".to_string(),
        full_name: "test-owner/test-repo".to_string(),
        description: Some("Test repository".to_string()),
        stars: 10,
        forks: 5,
        language: Some("Rust".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.save_repository(&repo).await?;
    
    // Create a test project
    let project = Project {
        id: ProjectId::new("PVT_kwDOAH9mBM4AAjJk".to_string()), // Sample GraphQL node ID
        owner: "test-owner".to_string(),
        number: ProjectNumber::new(1),
        title: "Sprint 2024-Q1".to_string(),
        description: Some("First quarter sprint planning".to_string()),
        state: "OPEN".to_string(),
        visibility: "PUBLIC".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        indexed_at: Utc::now(),
        linked_repositories: vec![repo.id],
    };
    db.save_project(project.clone()).await?;
    
    // Create a test issue
    let issue = Issue {
        id: IssueId::new(100),
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(1),
        title: "Test issue".to_string(),
        body: Some("This is a test issue".to_string()),
        state: IssueState::Open,
        author: "test-user".to_string(),
        assignees: vec!["test-user".to_string()],
        labels: vec!["bug".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 0,
        project_ids: vec![], // Initially empty
    };
    db.save_issue(&issue).await?;
    
    // Create a test PR
    let pr = PullRequest {
        id: PullRequestId::new(200),
        repository_id: repo.id,
        number: gitdb::ids::PullRequestNumber::new(2),
        title: "Test PR".to_string(),
        body: Some("This is a test PR".to_string()),
        state: PullRequestState::Open,
        author: "test-user".to_string(),
        assignees: vec![],
        labels: vec!["enhancement".to_string()],
        head_ref: "feature-branch".to_string(),
        base_ref: "main".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        merged_at: None,
        closed_at: None,
        comments_count: 0,
        commits_count: 1,
        additions: 10,
        deletions: 5,
        changed_files: 2,
        project_ids: vec![], // Initially empty
    };
    db.save_pull_request(&pr).await?;
    
    // Test 1: List all projects
    let projects = db.get_all_projects().await?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].title, "Sprint 2024-Q1");
    
    // Test 2: Add issue to project
    let project_item_issue = ProjectItem {
        id: format!("{}:{}:{}", project.id, ItemType::Issue, issue.id),
        project_id: project.id.clone(),
        item_type: ItemType::Issue,
        item_id: issue.id.value(),
        repository_id: repo.id,
        position: Some(1.0),
        added_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.add_item_to_project(project_item_issue).await?;
    db.add_project_to_issue(issue.id, project.id.clone()).await?;
    
    // Test 3: Add PR to project
    let project_item_pr = ProjectItem {
        id: format!("{}:{}:{}", project.id, ItemType::PullRequest, pr.id),
        project_id: project.id.clone(),
        item_type: ItemType::PullRequest,
        item_id: pr.id.value(),
        repository_id: repo.id,
        position: Some(2.0),
        added_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.add_item_to_project(project_item_pr).await?;
    db.add_project_to_pull_request(pr.id, project.id.clone()).await?;
    
    // Test 4: Get project items
    let items = db.get_project_items(&project.id).await?;
    assert_eq!(items.len(), 2);
    
    // Test 5: Get issues by project
    let project_issues = db.get_issues_by_project(&project.id).await?;
    assert_eq!(project_issues.len(), 1);
    assert_eq!(project_issues[0].title, "Test issue");
    
    // Test 6: Get PRs by project
    let project_prs = db.get_pull_requests_by_project(&project.id).await?;
    assert_eq!(project_prs.len(), 1);
    assert_eq!(project_prs[0].title, "Test PR");
    
    // Test 7: Check if issue is in project
    let is_in_project = db.is_issue_in_project(&issue.id, &project.id).await?;
    assert!(is_in_project);
    
    // Test 8: Get projects for issue
    let issue_projects = db.get_projects_for_issue(&issue.id).await?;
    assert_eq!(issue_projects.len(), 1);
    assert_eq!(issue_projects[0].title, "Sprint 2024-Q1");
    
    // Test 9: List issues filtered by project
    let filtered_issues = db.list_issues_filtered(None, Some(&project.id), None).await?;
    assert_eq!(filtered_issues.len(), 1);
    
    // Test 10: Get projects with stats
    let projects_with_stats = db.get_all_projects_with_stats().await?;
    assert_eq!(projects_with_stats.len(), 1);
    let (proj, issue_count, pr_count) = &projects_with_stats[0];
    assert_eq!(proj.title, "Sprint 2024-Q1");
    assert_eq!(*issue_count, 1);
    assert_eq!(*pr_count, 1);
    
    Ok(())
}