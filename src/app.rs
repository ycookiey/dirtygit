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
                SortMode::DirtyFirst => {
                    rb.is_dirty.cmp(&ra.is_dirty).then(ra.name.cmp(&rb.name))
                }
                SortMode::Name => ra.name.cmp(&rb.name),
                SortMode::LastCommit => {
                    // Compare by last_commit_time string (reverse for most recent first)
                    // This is a rough sort; exact sorting would require storing the timestamp
                    ra.last_commit_time.cmp(&rb.last_commit_time)
                }
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
}
