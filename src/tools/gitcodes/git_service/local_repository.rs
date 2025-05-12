use gix;
use gix::bstr::ByteSlice;
use gix::progress::Discard;
use rand::Rng;
use rmcp::schemars;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use thiserror::Error;

pub struct LocalRepository(PathBuf);
impl LocalRepository {
    /// Generate a unique directory name for the repository
    fn new_local_repository_to_clone(
        repository_cache_dir_base: &Path,
        user: &str,
        repo: &str,
    ) -> Repository {
        let random_suffix = rand::thread_rng().gen::<u32>() % 10000;
        let dir_name = format!("mcp_github_{}_{}_{}", user, repo, random_suffix);
        repository_cache_dir_base.join(dir_name)
    }

    /// Check if repository is already cloned
    //TODO(tacogips) rename to exists
    async fn is_local_repo_exists(&self, dir: &Path) -> bool {
        tokio::fs::metadata(dir).await.is_ok()
        //TODO(tacogips) check is git repository
    }

    /// List branches and tags for a GitHub repository or local git directory
    ///
    /// This tool retrieves a list of all branches and tags for the specified repository.
    /// It supports both public and private repositories as well as local git directories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    /// - For local directories: No authentication needed
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Clones or updates the repository locally (for GitHub URLs) or uses the local directory directly
    /// 2. Fetches all branches and tags
    /// 3. Formats the results into a readable format
    pub async fn list_repository_refs(&self, repository_location: RepositoryLocation) -> String {
        //// Parse repository information from URL or local path
        //let repo_info = match self
        //    .parse_and_prepare_repository(&repository_location ))
        //    .await
        //{
        //    Ok(info) => info,
        //    Err(e) => return e,
        //};

        //// Fetch repository refs using the extracted function
        //match fetch_repository_refs(&repo_info.repo_dir, &repo_info.user, &repo_info.repo).await {
        //    Ok(result) => result,
        //    Err(e) => format!("Failed to list refs: {}", e),
        //}
    }

    /// Update an existing repository
    ///
    /// Fetches the latest changes and checks out the specified branch/tag.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory containing the repository
    /// * `git_ref` - Branch or tag name to checkout as a GitRef
    async fn update_repository(&self, git_ref: &GitRef) -> Result<(), String> {
        let repo_dir = self.repository_cache_dir_base;
        // Open the existing repository
        let repo = gix::open(repo_dir).map_err(|e| format!("Failed to open repository: {}", e))?;

        // Find the origin remote
        let remote = repo
            .find_remote("origin")
            .map_err(|e| format!("Could not find origin remote: {}", e))?;

        // Configure fetch operation
        let depth = NonZeroU32::new(1).unwrap();
        let shallow = gix::remote::fetch::Shallow::DepthAtRemote(depth);

        // Prepare the fetch params
        let mut remote_ref_specs = Vec::new(); // Empty means fetch default refs
        let progress = Discard;

        // Create a transport for the fetch
        let transport = remote
            .connect(gix::remote::Direction::Fetch)
            .map_err(|e| format!("Failed to connect to remote: {}", e))?;

        // Create fetch delegate with our shallow config
        let mut delegate = transport.new_fetch_delegate();
        delegate.shallow_setting = Some(shallow);

        // Perform the fetch
        let fetch_outcome = delegate
            .fetch(&remote_ref_specs, &progress)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        // We don't need the fetch outcome details, just check for success
        let _ = fetch_outcome;

        // Try to find the reference directly (local branch)
        let local_ref_name = format!("refs/heads/{}", git_ref.as_str());
        let maybe_ref = repo.try_find_reference(&local_ref_name);
        if let Ok(Some(mut reference)) = maybe_ref {
            // Reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        // Try with origin/ prefix if direct reference wasn't found
        let origin_ref_name = format!("refs/remotes/origin/{}", git_ref.as_str());
        let maybe_origin_ref = repo.try_find_reference(&origin_ref_name);
        if let Ok(Some(mut reference)) = maybe_origin_ref {
            // Origin reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        // If we're looking for a tag
        let tag_ref_name = format!("refs/tags/{}", git_ref.as_str());
        let maybe_tag_ref = repo.try_find_reference(&tag_ref_name);
        if let Ok(Some(mut reference)) = maybe_tag_ref {
            // Tag reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        Err(format!("Branch/tag not found: {}", git_ref.as_str()))
    }
}
