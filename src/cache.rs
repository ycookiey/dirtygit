use crate::types::RepoStatus;
use std::fs;
use std::io;
use std::path::PathBuf;

fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dirtygit")
        .join("cache.json")
}

pub fn load_cache() -> Option<Vec<RepoStatus>> {
    let path = cache_path();
    let content = fs::read_to_string(&path).ok()?;
    let mut repos: Vec<RepoStatus> = serde_json::from_str(&content).ok()?;
    for r in &mut repos {
        r.stale = true;
    }
    Some(repos)
}

pub fn save_cache(repos: &[RepoStatus]) -> io::Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let fresh: Vec<&RepoStatus> = repos.iter().filter(|r| !r.stale).collect();

    let json = serde_json::to_string(&fresh)?;

    // Atomic write: write to temp file, then rename
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, &path)?;

    Ok(())
}
