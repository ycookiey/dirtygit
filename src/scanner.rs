use crate::config::Config;
use crate::git;
use crate::types::RepoStatus;
use std::path::Path;

pub fn scan_all(config: &Config) -> Vec<RepoStatus> {
    let mut repos = Vec::new();
    let scan_dirs = config.expanded_scan_dirs();
    let exclude = &config.exclude;
    let max_depth = config.max_depth;

    for dir in &scan_dirs {
        if dir.exists() {
            scan_directory(dir, exclude, max_depth, 0, &mut repos);
        }
    }

    repos
}

fn scan_directory(
    dir: &Path,
    exclude: &[String],
    max_depth: usize,
    current_depth: usize,
    repos: &mut Vec<RepoStatus>,
) {
    if current_depth > max_depth {
        return;
    }

    // Check if this directory itself is a git repo
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        if let Some(status) = git::get_repo_status(dir) {
            repos.push(status);
        }
        // Don't recurse into git repos (nested repos handled separately)
        return;
    }

    // Recurse into subdirectories
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

        // Skip hidden directories (except .git which is handled above)
        if dir_name.starts_with('.') {
            continue;
        }

        // Skip excluded directories
        if exclude.iter().any(|e| dir_name == *e) {
            continue;
        }

        scan_directory(&path, exclude, max_depth, current_depth + 1, repos);
    }
}
