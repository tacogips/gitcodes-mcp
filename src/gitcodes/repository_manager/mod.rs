pub mod providers;
mod repository_location;

use gitoxide_core as gix_core;
use std::{num::NonZeroU32, path::PathBuf, sync::atomic::AtomicBool};

use gix::{bstr::ByteSlice, progress::Discard};
use providers::GitRemoteRepository;
pub use repository_location::RepositoryLocation;
use tracing;

use crate::gitcodes::local_repository::LocalRepository;

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses a dedicated directory to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    pub github_token: Option<String>,
    pub local_repository_cache_dir_base: PathBuf,
}

impl RepositoryManager {
    /// Creates a new RepositoryManager instance with a custom repository cache directory
    ///
    /// # Parameters
    ///
    /// * `repository_cache_dir` - Optional custom path for storing repositories.
    ///                            If None, the system's temporary directory is used.
    ///
    /// # Returns
    ///
    /// * `Result<Self, String>` - A new RepositoryManager instance or an error message
    ///                            if the directory cannot be created or accessed.
    pub fn new(
        github_token: Option<String>,
        local_repository_cache_dir_base: Option<PathBuf>,
    ) -> Result<Self, String> {
        // Use provided path or default to system temp directory
        let local_repository_cache_dir_base = match local_repository_cache_dir_base {
            Some(path) => path,
            None => std::env::temp_dir(),
        };

        // Validate and ensure the directory exists
        if !local_repository_cache_dir_base.exists() {
            // Try to create the directory if it doesn't exist
            std::fs::create_dir_all(&local_repository_cache_dir_base)
                .map_err(|e| format!("Failed to create repository cache directory: {}", e))?;
        } else if !local_repository_cache_dir_base.is_dir() {
            return Err(format!(
                "Specified path '{}' is not a directory",
                local_repository_cache_dir_base.display()
            ));
        }

        Ok(Self {
            github_token,
            local_repository_cache_dir_base,
        })
    }

    /// Creates a new RepositoryManager with the system's default cache directory
    ///
    /// This is a convenience method that creates a RepositoryManager with the
    /// system's temporary directory as the repository cache location.
    pub fn with_default_cache_dir() -> Self {
        Self::new(None, None).expect("Failed to initialize with system temporary directory")
    }

    pub async fn prepare_repository(
        &self,
        repo_location: RepositoryLocation,
        ref_name: Option<String>,
    ) -> Result<LocalRepository, String> {
        match repo_location {
            RepositoryLocation::LocalPath(local_path) => {
                local_path.validate()?;
                Ok(local_path)
            }
            RepositoryLocation::RemoteRepository(mut remote_repository) => {
                // If a specific ref_name was provided to this function, update the repository info
                if let Some(ref_name_str) = ref_name {
                    // Update the remote repository with the provided ref_name
                    match remote_repository {
                        GitRemoteRepository::Github(ref mut github_info) => {
                            github_info.repo_info.ref_name = Some(ref_name_str);
                        }
                    }
                }

                self.clone_repository(&remote_repository).await
            }
        }
    }

    /// Clone a repository from GitHub
    ///
    /// Creates a directory and performs a shallow clone of the specified repository.
    /// Uses a structured RemoteGitRepositoryInfo object to encapsulate all required clone parameters.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory where the repository should be cloned
    /// * `params` - RemoteGitRepositoryInfo struct containing user, repo, and ref_name
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Success or an error message if the clone operation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::tools::gitcodes::git_service::git_repository::{clone_repository, RemoteGitRepositoryInfo};
    ///
    /// async fn example() {
    ///     let repo_dir = PathBuf::from("/tmp/example_repo");
    ///     let params = RemoteGitRepositoryInfo {
    ///         user: "rust-lang".to_string(),
    ///         repo: "rust".to_string(),
    ///         ref_name: "main".to_string(),
    ///     };
    ///
    ///     match clone_repository(&repo_dir, &params).await {
    ///         Ok(()) => println!("Repository cloned successfully"),
    ///         Err(e) => eprintln!("Failed to clone repository: {}", e),
    ///     }
    /// }
    /// ```
    async fn clone_repository(
        &self,
        remote_repository: &GitRemoteRepository,
    ) -> Result<LocalRepository, String> {
        use gix::{
            clone::PrepareFetch,
            create::{Kind, Options as CreateOptions},
            open::Options as OpenOptions,
            progress::Discard,
        };
        use std::sync::atomic::AtomicBool;

        // Create a unique local repository directory based on the remote repository info
        let local_repo = LocalRepository::new_local_repository_to_clone(match remote_repository {
            GitRemoteRepository::Github(github_info) => github_info.repo_info.clone(),
        });

        // Ensure the destination directory doesn't exist already
        let repo_dir = local_repo.get_repository_dir();
        if repo_dir.exists() {
            if repo_dir.is_dir() {
                // Repository already exists, let's validate it
                match local_repo.validate() {
                    Ok(_) => {
                        tracing::info!(
                            "Repository already exists at {}, reusing it",
                            repo_dir.display()
                        );
                        return Ok(local_repo);
                    }
                    Err(e) => {
                        // Directory exists but is not a valid repository, clean it up
                        tracing::warn!(
                            "Found invalid repository at {}, removing it: {}",
                            repo_dir.display(),
                            e
                        );
                        if let Err(e) = std::fs::remove_dir_all(repo_dir) {
                            return Err(format!(
                                "Failed to remove invalid repository directory: {}",
                                e
                            ));
                        }
                    }
                }
            } else {
                return Err(format!(
                    "Destination path exists but is not a directory: {}",
                    repo_dir.display()
                ));
            }
        }

        //
        //pub fn clone<P>(
        //    url: impl AsRef<OsStr>,
        //    directory: Option<impl Into<std::path::PathBuf>>,
        //    overrides: Vec<BString>,
        //    mut progress: P,
        //    mut out: impl std::io::Write,
        //    mut err: impl std::io::Write,
        //    Options {
        //        format,
        //        handshake_info,
        //        bare,
        //        no_tags,
        //        ref_name,
        //        shallow,
        //    }: Options,
        //) -> anyhow::Result<()>
        //where
        //    P: NestedProgress,
        //    P::SubProgress: 'static,
        //{
        //    if format != OutputFormat::Human {
        //        bail!("JSON output isn't yet supported for fetching.");
        //    }

        //    let url: gix::Url = url.as_ref().try_into()?;
        //    let directory = directory.map_or_else(
        //        || {
        //            let path = gix::path::from_bstr(Cow::Borrowed(url.path.as_ref()));
        //            if !bare && path.extension() == Some(OsStr::new("git")) {
        //                path.file_stem().map(Into::into)
        //            } else {
        //                path.file_name().map(Into::into)
        //            }
        //            .context("Filename extraction failed - path too short")
        //        },
        //        |dir| Ok(dir.into()),
        //    )?;
        //    let mut prepare = gix::clone::PrepareFetch::new(
        //        url,
        //        directory,
        //        if bare {
        //            gix::create::Kind::Bare
        //        } else {
        //            gix::create::Kind::WithWorktree
        //        },
        //        gix::create::Options::default(),
        //        {
        //            let mut opts = gix::open::Options::default().config_overrides(overrides);
        //            opts.permissions.config.git_binary = true;
        //            opts
        //        },
        //    )?;
        //    if no_tags {
        //        prepare = prepare.configure_remote(|r| Ok(r.with_fetch_tags(gix::remote::fetch::Tags::None)));
        //    }
        //    let (mut checkout, fetch_outcome) = prepare
        //        .with_shallow(shallow)
        //        .with_ref_name(ref_name.as_ref())?
        //        .fetch_then_checkout(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

        //    let (repo, outcome) = if bare {
        //        (checkout.persist(), None)
        //    } else {
        //        let (repo, outcome) = checkout.main_worktree(progress, &gix::interrupt::IS_INTERRUPTED)?;
        //        (repo, Some(outcome))
        //    };

        //    if handshake_info {
        //        writeln!(out, "Handshake Information")?;
        //        writeln!(out, "\t{:?}", fetch_outcome.handshake)?;
        //    }

        //    match fetch_outcome.status {
        //        Status::NoPackReceived { dry_run, .. } => {
        //            assert!(!dry_run, "dry-run unsupported");
        //            writeln!(err, "The cloned repository appears to be empty")?;
        //        }
        //        Status::Change {
        //            update_refs, negotiate, ..
        //        } => {
        //            let remote = repo
        //                .find_default_remote(gix::remote::Direction::Fetch)
        //                .expect("one origin remote")?;
        //            let ref_specs = remote.refspecs(gix::remote::Direction::Fetch);
        //            print_updates(
        //                &repo,
        //                &negotiate,
        //                update_refs,
        //                ref_specs,
        //                fetch_outcome.ref_map,
        //                &mut out,
        //                &mut err,
        //            )?;
        //        }
        //    }

        //    if let Some(gix::worktree::state::checkout::Outcome { collisions, errors, .. }) = outcome {
        //        if !(collisions.is_empty() && errors.is_empty()) {
        //            let mut messages = Vec::new();
        //            if !errors.is_empty() {
        //                messages.push(format!("kept going through {} errors(s)", errors.len()));
        //                for record in errors {
        //                    writeln!(err, "{}: {}", record.path, record.error).ok();
        //                }
        //            }
        //            if !collisions.is_empty() {
        //                messages.push(format!("encountered {} collision(s)", collisions.len()));
        //                for col in collisions {
        //                    writeln!(err, "{}: collision ({:?})", col.path, col.error_kind).ok();
        //                }
        //            }
        //            bail!(
        //                "One or more errors occurred - checkout is incomplete: {}",
        //                messages.join(", ")
        //            );
        //        }
        //    }
        //    Ok(())
        //}
        gix_core::repository::clone(remote, directory, config, progress, out, err, opts)?;
        let local_repository = ???;
        Ok(())
        //// Create parent directory if it doesn't exist
        //if let Some(parent) = repo_dir.parent() {
        //    if !parent.exists() {
        //        if let Err(e) = std::fs::create_dir_all(parent) {
        //            return Err(format!("Failed to create parent directories: {}", e));
        //        }
        //    }
        //}

        //// Get the URL from the remote repository
        //let clone_url = remote_repository.clone_url();
        //let ref_name = remote_repository.get_ref_name();
        //
        //tracing::info!(
        //    "Cloning repository from {} to {}{}",
        //    clone_url,
        //    repo_dir.display(),
        //    ref_name
        //        .as_ref()
        //        .map(|r| format!(" (ref: {})", r))
        //        .unwrap_or_default()
        //);

        //// Configure git repository creation options
        //let create_opts = CreateOptions::default();
        //let open_opts = OpenOptions::default();
        //
        //// Initialize a repo for fetching
        //let mut fetch = match PrepareFetch::new(
        //    &clone_url,
        //    repo_dir,
        //    Kind::WorkTree,
        //    create_opts,
        //    open_opts,
        //) {
        //    Ok(fetch) => fetch,
        //    Err(e) => return Err(format!("Failed to prepare repository for fetching: {}", e)),
        //};

        //// Configure the reference to fetch if specified
        //if let Some(ref_name) = ref_name {
        //    fetch = match fetch.with_ref_name(Some(ref_name)) {
        //        Ok(f) => f,
        //        Err(e) => return Err(format!("Invalid reference name: {}", e)),
        //    };
        //}

        //// Add GitHub authentication token if available
        //if let Some(token) = &self.github_token {
        //    if let GitRemoteRepository::Github(_) = remote_repository {
        //        let token_clone = token.clone();
        //        fetch = fetch.configure_remote(move |remote| {
        //            // Add GitHub authentication if we have a token
        //            // This sets the authentication for the remote URL
        //            if let Ok(url) = remote.url().to_string().parse::<gix_url::Url>() {
        //                if url.scheme().starts_with("http") {
        //                    let mut url = url;
        //                    // Add the token to the URL
        //                    if let Some(user_info) = url.user_info_mut() {
        //                        *user_info = format!("{}:", token_clone);
        //                    }
        //                    return Ok(remote.with_url(url.to_string())
        //                        .expect("URL with token should be valid"));
        //                }
        //            }
        //            Ok(remote)
        //        });
        //    }
        //}

        //// Clone the repository
        //let should_interrupt = AtomicBool::new(false);
        //match fetch.fetch_only(Discard, &should_interrupt) {
        //    Ok((repository, _outcome)) => {
        //        tracing::info!("Successfully cloned repository to {}", repo_dir.display());
        //        Ok(local_repo)
        //    }
        //    Err(e) => {
        //        // Clean up failed clone attempt
        //        if repo_dir.exists() {
        //            let _ = std::fs::remove_dir_all(repo_dir);
        //        }
        //        Err(format!("Failed to clone repository: {}", e))
        //    }
        //}
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
