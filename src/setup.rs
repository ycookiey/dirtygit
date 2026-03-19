use crossterm::event::{KeyCode, KeyModifiers};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupFocus {
    Input,
    List,
}

pub enum SetupAction {
    None,
    Quit,
    Save(Vec<String>),
}

pub struct CompletionState {
    pub candidates: Vec<String>,
    pub selected: usize,
    pub menu_open: bool,
    pub base_dir: String,
}

impl CompletionState {
    fn new() -> Self {
        Self {
            candidates: Vec::new(),
            selected: 0,
            menu_open: false,
            base_dir: String::new(),
        }
    }

    fn clear(&mut self) {
        self.candidates.clear();
        self.selected = 0;
        self.menu_open = false;
        self.base_dir.clear();
    }

    fn close(&mut self) {
        self.menu_open = false;
        self.selected = 0;
    }

    /// Split input into (base_dir, prefix) at the last path separator.
    /// Expands `~` to home directory.
    fn split_input(input: &str) -> (String, String) {
        let expanded = expand_tilde(input);
        // Find last separator
        let last_sep = expanded
            .rfind('/')
            .or_else(|| expanded.rfind('\\'));
        match last_sep {
            Some(pos) => {
                let base = &expanded[..=pos];
                let prefix = &expanded[pos + 1..];
                (base.to_string(), prefix.to_string())
            }
            None => (String::new(), expanded),
        }
    }

    /// Read directory entries and filter by prefix.
    fn read_candidates(base_dir: &str, prefix: &str) -> Vec<String> {
        let dir_path = if base_dir.is_empty() {
            return Vec::new();
        } else {
            Path::new(base_dir)
        };

        let Ok(entries) = std::fs::read_dir(dir_path) else {
            return Vec::new();
        };

        let show_hidden = prefix.starts_with('.');
        let prefix_lower = prefix.to_lowercase();

        let mut candidates: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if !show_hidden && name.starts_with('.') {
                    return None;
                }
                if name.to_lowercase().starts_with(&prefix_lower) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        candidates.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        candidates
    }

    /// Find the longest common prefix among candidates.
    fn common_prefix(candidates: &[String]) -> String {
        if candidates.is_empty() {
            return String::new();
        }
        let first = &candidates[0];
        let mut len = first.len();
        for c in &candidates[1..] {
            len = len.min(c.len());
            for (i, (a, b)) in first.chars().zip(c.chars()).enumerate() {
                if a.to_lowercase().to_string() != b.to_lowercase().to_string() {
                    len = len.min(i);
                    break;
                }
            }
        }
        first[..len].to_string()
    }

    /// Tab pressed: trigger completion or cycle through candidates.
    pub fn trigger(&mut self, input: &mut String) {
        if self.menu_open {
            self.cycle(input);
            return;
        }

        let (base, prefix) = Self::split_input(input);
        let candidates = Self::read_candidates(&base, &prefix);

        self.base_dir = base.clone();
        self.candidates = candidates;
        self.selected = 0;

        match self.candidates.len() {
            0 => {}
            1 => {
                // Single match: auto-complete
                let sep = std::path::MAIN_SEPARATOR;
                *input = format!("{}{}{}", base, self.candidates[0], sep);
                self.clear();
            }
            _ => {
                // Multiple matches: complete common prefix and open menu
                let common = Self::common_prefix(&self.candidates);
                if common.len() > prefix.len() {
                    *input = format!("{}{}", base, common);
                }
                self.menu_open = true;
                self.selected = 0;
            }
        }
    }

    /// Cycle to next candidate in menu.
    fn cycle(&mut self, input: &mut String) {
        if self.candidates.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.candidates.len();
        let sep = std::path::MAIN_SEPARATOR;
        *input = format!("{}{}{}", self.base_dir, self.candidates[self.selected], sep);
    }

    /// Re-filter candidates after user types more characters.
    pub fn refilter(&mut self, input: &mut String) {
        if !self.menu_open {
            return;
        }

        let (base, prefix) = Self::split_input(input);

        // If base_dir changed, close the menu (user navigated to different dir)
        if base != self.base_dir {
            self.close();
            return;
        }

        let candidates = Self::read_candidates(&base, &prefix);
        self.candidates = candidates;

        match self.candidates.len() {
            0 => self.close(),
            1 => {
                let sep = std::path::MAIN_SEPARATOR;
                *input = format!("{}{}{}", base, self.candidates[0], sep);
                self.clear();
            }
            _ => {
                self.selected = 0;
            }
        }
    }

    /// Accept current selection, return the completed path.
    pub fn accept(&self) -> Option<String> {
        if self.menu_open && !self.candidates.is_empty() {
            let sep = std::path::MAIN_SEPARATOR;
            Some(format!(
                "{}{}{}",
                self.base_dir, self.candidates[self.selected], sep
            ))
        } else {
            None
        }
    }
}

pub struct SetupState {
    pub input: String,
    pub dirs: Vec<String>,
    pub selected_dir: usize,
    pub focus: SetupFocus,
    pub completion: CompletionState,
    pub warning: Option<String>,
}

impl SetupState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            dirs: Vec::new(),
            selected_dir: 0,
            focus: SetupFocus::Input,
            completion: CompletionState::new(),
            warning: None,
        }
    }

    fn add_current_input(&mut self) {
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let expanded = expand_tilde(&trimmed);

        // Strip trailing separator
        let path_str = expanded
            .trim_end_matches('/')
            .trim_end_matches('\\')
            .to_string();

        // Duplicate check (case-insensitive on Windows)
        if self
            .dirs
            .iter()
            .any(|d| d.eq_ignore_ascii_case(&path_str))
        {
            self.warning = Some("already added".to_string());
            return;
        }

        // Existence check
        if !Path::new(&path_str).is_dir() {
            self.warning = Some("path does not exist (added anyway)".to_string());
        }

        self.dirs.push(path_str);
        self.input.clear();
        self.completion.clear();
    }

    fn delete_selected(&mut self) {
        if self.dirs.is_empty() {
            return;
        }
        self.dirs.remove(self.selected_dir);
        if self.selected_dir >= self.dirs.len() && self.selected_dir > 0 {
            self.selected_dir -= 1;
        }
        if self.dirs.is_empty() {
            self.focus = SetupFocus::Input;
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> SetupAction {
        // Ctrl+C: quit
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return SetupAction::Quit;
        }

        // Clear warning on any keypress
        self.warning = None;

        match self.focus {
            SetupFocus::Input => self.handle_input_key(code, modifiers),
            SetupFocus::List => self.handle_list_key(code),
        }
    }

    fn handle_input_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> SetupAction {
        match code {
            KeyCode::Char(c) => {
                self.input.push(c);
                self.completion.refilter(&mut self.input);
                SetupAction::None
            }
            KeyCode::Backspace => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    // Delete back to last path separator
                    let trimmed = self.input.trim_end_matches(['/', '\\']);
                    match trimmed.rfind(['/', '\\']) {
                        Some(pos) => self.input.truncate(pos + 1),
                        None => self.input.clear(),
                    }
                } else {
                    self.input.pop();
                }
                if self.completion.menu_open {
                    self.completion.refilter(&mut self.input);
                }
                SetupAction::None
            }
            KeyCode::Tab => {
                if self.input.is_empty() {
                    // Empty input: start from parent of current directory
                    let start = std::env::current_dir()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .or_else(dirs::home_dir);
                    if let Some(dir) = start {
                        let sep = std::path::MAIN_SEPARATOR;
                        self.input = format!("{}{}", dir.display(), sep);
                    }
                }
                self.completion.trigger(&mut self.input);
                SetupAction::None
            }
            KeyCode::Enter => {
                if self.completion.menu_open {
                    // Accept completion selection
                    if let Some(path) = self.completion.accept() {
                        self.input = path;
                    }
                    self.completion.close();
                } else if self.input.trim().is_empty() {
                    // Empty Enter → save
                    if !self.dirs.is_empty() {
                        return SetupAction::Save(self.dirs.clone());
                    }
                } else {
                    // Add to list
                    self.add_current_input();
                }
                SetupAction::None
            }
            KeyCode::Esc => {
                if self.completion.menu_open {
                    self.completion.close();
                }
                SetupAction::None
            }
            _ => SetupAction::None,
        }
    }

    fn handle_list_key(&mut self, code: KeyCode) -> SetupAction {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.dirs.is_empty() && self.selected_dir < self.dirs.len() - 1 {
                    self.selected_dir += 1;
                }
                SetupAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_dir > 0 {
                    self.selected_dir -= 1;
                }
                SetupAction::None
            }
            KeyCode::Char('d') => {
                self.delete_selected();
                SetupAction::None
            }
            KeyCode::Tab => {
                self.focus = SetupFocus::Input;
                SetupAction::None
            }
            KeyCode::Enter => {
                if !self.dirs.is_empty() {
                    return SetupAction::Save(self.dirs.clone());
                }
                SetupAction::None
            }
            _ => SetupAction::None,
        }
    }
}

fn expand_tilde(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = dirs::home_dir() {
            let rest = if path == "~" { "" } else { &path[2..] };
            return format!("{}{}{}", home.display(), std::path::MAIN_SEPARATOR, rest);
        }
    }
    path.to_string()
}
