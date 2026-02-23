use crate::types::{FileChange, RepoStatus};
use chrono::{DateTime, FixedOffset, Utc};
use std::path::Path;
use std::process::Command;

pub fn get_repo_status(path: &Path) -> Option<RepoStatus> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Get branch and status info using porcelain v2
    let status_output = Command::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .current_dir(path)
        .output()
        .ok()?;

    if !status_output.status.success() {
        return None;
    }

    let status_text = String::from_utf8_lossy(&status_output.stdout);

    let mut branch = String::from("HEAD");
    let mut ahead: u32 = 0;
    let mut behind: u32 = 0;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for line in status_text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: +<ahead> -<behind>
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                ahead = parts[0].trim_start_matches('+').parse().unwrap_or(0);
                behind = parts[1].trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            // Changed entry: "1 XY ..." or "2 XY ... <path><tab><origPath>"
            let parts: Vec<&str> = line.splitn(9, ' ').collect();
            if parts.len() >= 9 {
                let xy = parts[1];
                let x = xy.chars().next().unwrap_or('.');
                let y = xy.chars().nth(1).unwrap_or('.');

                // For rename entries (type 2), path might contain a tab
                let file_path = if line.starts_with("2 ") {
                    // Rename: last field is "path\torigPath"
                    parts[8].split('\t').next().unwrap_or(parts[8]).to_string()
                } else {
                    parts[8].to_string()
                };

                if x != '.' {
                    staged.push(FileChange {
                        status: x,
                        path: file_path.clone(),
                    });
                }
                if y != '.' {
                    unstaged.push(FileChange {
                        status: y,
                        path: file_path,
                    });
                }
            }
        } else if let Some(rest) = line.strip_prefix("? ") {
            untracked.push(rest.to_string());
        }
    }

    // Get last commit info
    let (last_commit_message, last_commit_time) = get_last_commit(path);

    let is_dirty = !staged.is_empty() || !unstaged.is_empty() || !untracked.is_empty();

    Some(RepoStatus {
        name,
        path: path.to_path_buf(),
        branch,
        is_dirty,
        staged,
        unstaged,
        untracked,
        ahead,
        behind,
        last_commit_message,
        last_commit_time,
    })
}

fn get_last_commit(path: &Path) -> (String, String) {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%s%n%aI"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = text.lines().collect();
            let message = lines.first().unwrap_or(&"").to_string();
            let time_str = lines.get(1).unwrap_or(&"");
            let relative = format_relative_time(time_str);
            (message, relative)
        }
        _ => ("(no commits)".to_string(), "".to_string()),
    }
}

fn format_relative_time(iso_str: &str) -> String {
    let dt = match DateTime::<FixedOffset>::parse_from_rfc3339(iso_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return iso_str.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    let secs = duration.num_seconds();
    if secs < 0 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if secs < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days < 30 {
        format!("{}d ago", days)
    } else if days < 365 {
        format!("{}mo ago", days / 30)
    } else {
        format!("{}y ago", days / 365)
    }
}
