use crate::app::SortMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Preferences {
    pub sort_mode: SortMode,
    pub dirty_filter: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sort_mode: SortMode::DirtyFirst,
            dirty_filter: false,
        }
    }
}

fn prefs_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dirtygit")
        .join("preferences.json")
}

pub fn load() -> Preferences {
    let path = prefs_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(prefs: &Preferences) {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(prefs) {
        let _ = std::fs::write(&path, json);
    }
}
