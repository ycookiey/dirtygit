use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_scan_dirs")]
    pub scan_dirs: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_scan_dirs() -> Vec<String> {
    vec!["~/projects".to_string()]
}

fn default_interval() -> u64 {
    300
}

fn default_max_depth() -> usize {
    3
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_dirs: default_scan_dirs(),
            exclude: Vec::new(),
            interval_secs: default_interval(),
            max_depth: default_max_depth(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("git-all-status")
            .join("config.toml")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Err(format!(
                "設定ファイルが見つかりません。{} を作成してください\n\n\
                 サンプル設定:\n\n\
                 # ~/.config/git-all-status/config.toml\n\
                 scan_dirs = [\n\
                     \"~/projects\",\n\
                     \"~/work\",\n\
                 ]\n\
                 exclude = [\"node_modules\", \".cache\", \"vendor\"]\n\
                 interval_secs = 300\n\
                 max_depth = 3\n",
                path.display()
            ));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("設定ファイルの読み込みに失敗: {}", e))?;

        let config: Config =
            toml::from_str(&content).map_err(|e| format!("設定ファイルのパースに失敗: {}", e))?;

        Ok(config)
    }

    pub fn expanded_scan_dirs(&self) -> Vec<PathBuf> {
        self.scan_dirs
            .iter()
            .map(|d| {
                if d.starts_with("~/") || d == "~" {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&d[2..])
                    } else {
                        PathBuf::from(d)
                    }
                } else {
                    PathBuf::from(d)
                }
            })
            .collect()
    }
}
