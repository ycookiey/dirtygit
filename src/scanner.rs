use crate::config::Config;
use crate::event::Event;
use crate::git;
use crate::types::RepoStatus;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

/// Discover repo paths only (no git commands), then scan each in parallel.
/// `cached_repos` is used to prioritize dirty repos first.
pub async fn scan_all_parallel(
    config: &Config,
    tx: mpsc::UnboundedSender<Event>,
    cached_repos: &[RepoStatus],
) {
    let mut paths = discover_repos(config);

    let dirty_paths: HashSet<PathBuf> = cached_repos
        .iter()
        .filter(|r| r.is_dirty)
        .map(|r| r.path.clone())
        .collect();
    paths.sort_by_key(|p| if dirty_paths.contains(p) { 0 } else { 1 });

    let semaphore = Arc::new(Semaphore::new(8));

    let mut handles = Vec::new();
    for path in paths {
        let sem = semaphore.clone();
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let status =
                tokio::task::spawn_blocking(move || git::get_repo_status(&path)).await;
            if let Ok(Some(status)) = status {
                let _ = tx.send(Event::RepoUpdated(Box::new(status)));
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
    let _ = tx.send(Event::ScanComplete);
}

/// Walk scan_dirs and collect paths that contain .git (no git commands executed).
fn discover_repos(config: &Config) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let scan_dirs = config.expanded_scan_dirs();
    let exclude = &config.exclude;
    let max_depth = config.max_depth;

    for dir in &scan_dirs {
        if dir.exists() {
            walk_for_repos(dir, exclude, max_depth, 0, &mut paths);
        }
    }
    paths
}

fn walk_for_repos(
    dir: &Path,
    exclude: &[String],
    max_depth: usize,
    current_depth: usize,
    paths: &mut Vec<PathBuf>,
) {
    if current_depth > max_depth {
        return;
    }

    let git_dir = dir.join(".git");
    if git_dir.exists() {
        paths.push(dir.to_path_buf());
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if dir_name.starts_with('.') {
            continue;
        }

        if exclude.contains(&dir_name) {
            continue;
        }

        walk_for_repos(&path, exclude, max_depth, current_depth + 1, paths);
    }
}
