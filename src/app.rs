use crate::types::RepoStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    DirtyFirst,
    Name,
    LastCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    RepoList,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

pub struct App {
    pub running: bool,
    pub repos: Vec<RepoStatus>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub detail_scroll: usize,
    pub active_pane: ActivePane,
    pub sort_mode: SortMode,
    pub dirty_filter: bool,
    pub input_mode: InputMode,
    pub search_query: String,
    pub scanning: bool,
    pub last_scan_time: Option<String>,
    pub config_error: Option<String>,
    pub list_height: usize, // visible rows in repo list (updated each frame)
    pub show_help: bool,
    pub flash_message: Option<(String, std::time::Instant)>,
    pub repo_list_area: (u16, u16, u16, u16), // (x, y, width, height) for mouse hit testing
    pub pending_external: Option<std::path::PathBuf>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            repos: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            detail_scroll: 0,
            active_pane: ActivePane::RepoList,
            sort_mode: SortMode::DirtyFirst,
            dirty_filter: false,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            scanning: false,
            last_scan_time: None,
            config_error: None,
            list_height: 20,
            show_help: false,
            flash_message: None,
            repo_list_area: (0, 0, 0, 0),
            pending_external: None,
        }
    }

    pub fn update_filtered(&mut self) {
        self.filtered_indices = (0..self.repos.len())
            .filter(|&i| {
                let repo = &self.repos[i];
                if self.dirty_filter && !repo.is_dirty {
                    return false;
                }
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !repo.name.to_lowercase().contains(&query) {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Sort filtered indices
        let repos = &self.repos;
        let sort_mode = self.sort_mode;
        self.filtered_indices.sort_by(|&a, &b| {
            let ra = &repos[a];
            let rb = &repos[b];
            match sort_mode {
                SortMode::DirtyFirst => rb
                    .is_dirty
                    .cmp(&ra.is_dirty)
                    .then(rb.last_commit_epoch.cmp(&ra.last_commit_epoch)),
                SortMode::Name => ra.name.cmp(&rb.name),
                SortMode::LastCommit => rb.last_commit_epoch.cmp(&ra.last_commit_epoch),
            }
        });

        // Clamp selection
        if !self.filtered_indices.is_empty() {
            if self.selected >= self.filtered_indices.len() {
                self.selected = self.filtered_indices.len() - 1;
            }
        } else {
            self.selected = 0;
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoStatus> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.repos.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_up_n(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
        self.detail_scroll = 0;
    }

    pub fn move_down_n(&mut self, n: usize) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + n).min(self.filtered_indices.len() - 1);
            self.detail_scroll = 0;
        }
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::DirtyFirst => SortMode::Name,
            SortMode::Name => SortMode::LastCommit,
            SortMode::LastCommit => SortMode::DirtyFirst,
        };
        self.update_filtered();
    }

    pub fn toggle_dirty_filter(&mut self) {
        self.dirty_filter = !self.dirty_filter;
        self.update_filtered();
    }

    pub fn set_repos(&mut self, repos: Vec<RepoStatus>) {
        self.repos = repos;
        self.update_filtered();
    }

    /// Load cached repos with stale=true.
    pub fn load_from_cache(&mut self, mut repos: Vec<RepoStatus>) {
        for r in &mut repos {
            r.stale = true;
        }
        self.set_repos(repos);
    }

    /// Update a single repo by path match, or add if new.
    pub fn update_repo(&mut self, status: RepoStatus) {
        if let Some(pos) = self.repos.iter().position(|r| r.path == status.path) {
            self.repos[pos] = status;
        } else {
            self.repos.push(status);
        }
        self.update_filtered();
    }

    pub fn stale_count(&self) -> usize {
        self.repos.iter().filter(|r| r.stale).count()
    }
}
