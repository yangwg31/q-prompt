use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub use_count: u32,
    pub last_used: u64,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub display_mode: String,
    pub ide_list: Vec<String>,
    pub bar_position: Position,
    pub shortcuts: HashMap<String, String>,
    pub launch_on_startup: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut shortcuts = HashMap::new();
        shortcuts.insert("toggle_bar".into(), "Alt+Q".into());
        shortcuts.insert("insert_1".into(), "Alt+1".into());
        shortcuts.insert("insert_2".into(), "Alt+2".into());
        shortcuts.insert("insert_3".into(), "Alt+3".into());
        shortcuts.insert("insert_4".into(), "Alt+4".into());
        shortcuts.insert("insert_5".into(), "Alt+5".into());
        shortcuts.insert("insert_6".into(), "Alt+6".into());
        shortcuts.insert("insert_7".into(), "Alt+7".into());
        shortcuts.insert("insert_8".into(), "Alt+8".into());
        shortcuts.insert("quick_save".into(), "Alt+S".into());

        Self {
            display_mode: "always".into(),
            ide_list: vec![
                "Code.exe".into(),
                "idea64.exe".into(),
                "webstorm64.exe".into(),
                "pycharm64.exe".into(),
                "qstudio.exe".into(),
                "devenv.exe".into(),
            ],
            bar_position: Position { x: -1, y: -1 },
            shortcuts,
            launch_on_startup: false,
        }
    }
}

pub struct PromptStore {
    data_dir: PathBuf,
}

impl PromptStore {
    pub fn new() -> Self {
        let data_dir = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&data_dir).ok();
        Self { data_dir }
    }

    pub fn prompts_path(&self) -> PathBuf {
        self.data_dir.join("prompts.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }

    pub fn deleted_backup_path(&self) -> PathBuf {
        self.data_dir.join("deleted_backup.json")
    }

    pub fn load_prompts(&self) -> Vec<PromptItem> {
        let path = self.prompts_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
                Err(e) => {
                    self.backup_damaged_file(&path);
                    log::error!("Failed to read prompts.json: {}, rebuilding default", e);
                    let defaults = Self::default_prompts();
                    self.save_prompts(&defaults);
                    defaults
                }
            }
        } else {
            let defaults = Self::default_prompts();
            self.save_prompts(&defaults);
            defaults
        }
    }

    pub fn save_prompts(&self, items: &[PromptItem]) {
        let path = self.prompts_path();
        if let Ok(data) = serde_json::to_string_pretty(items) {
            fs::write(&path, data).unwrap_or_else(|e| {
                log::error!("Failed to write prompts.json: {}", e);
            });
        }
    }

    pub fn load_config(&self) -> Config {
        let path = self.config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
                Err(e) => {
                    self.backup_damaged_file(&path);
                    log::error!("Failed to read config.json: {}, rebuilding default", e);
                    let defaults = Config::default();
                    self.save_config(&defaults);
                    defaults
                }
            }
        } else {
            let defaults = Config::default();
            self.save_config(&defaults);
            defaults
        }
    }

    pub fn save_config(&self, config: &Config) {
        let path = self.config_path();
        if let Ok(data) = serde_json::to_string_pretty(config) {
            fs::write(&path, data).unwrap_or_else(|e| {
                log::error!("Failed to write config.json: {}", e);
            });
        }
    }

    pub fn load_deleted_backup(&self) -> Vec<PromptItem> {
        let path = self.deleted_backup_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
                Err(_) => {
                    self.save_deleted_backup(&[]);
                    vec![]
                }
            }
        } else {
            self.save_deleted_backup(&[]);
            vec![]
        }
    }

    pub fn save_deleted_backup(&self, items: &[PromptItem]) {
        let path = self.deleted_backup_path();
        if let Ok(data) = serde_json::to_string_pretty(items) {
            fs::write(&path, data).ok();
        }
    }

    fn backup_damaged_file(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            if let Some(name) = path.file_stem() {
                let bak = parent.join(format!("{}.bak", name.to_string_lossy()));
                fs::rename(path, &bak).ok();
            }
        }
    }

    fn default_prompts() -> Vec<PromptItem> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        vec![
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "复核代码".into(),
                content: "请仔细复核以下代码，指出潜在问题并给出改进建议：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 0,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "生成测试".into(),
                content: "请为以下函数编写单元测试，覆盖正常路径和边界情况：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 1,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "添加注释".into(),
                content: "请为以下代码添加清晰的中文注释，说明关键逻辑：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 2,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "优化性能".into(),
                content: "请分析以下代码的性能瓶颈并提出优化方案：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 3,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "解释代码".into(),
                content: "请用通俗易懂的语言解释以下代码的功能和执行流程：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 4,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "修复Bug".into(),
                content: "以下代码存在 bug，请帮我定位并修复：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 5,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "生成文档".into(),
                content: "请为以下代码生成 API 文档 / 使用说明：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 6,
            },
            PromptItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "代码转换".into(),
                content: "请将以下代码从 {{源语言}} 转换为 {{目标语言}}：\n\n```\n{{selection}}\n```".into(),
                use_count: 0, last_used: now, sort_order: 7,
            },
        ]
    }
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("q-prompt"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs_next_mac()
    }
    #[cfg(target_os = "linux")]
    {
        dirs_next_linux()
    }
}

#[cfg(target_os = "macos")]
fn dirs_next_mac() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|p| PathBuf::from(p).join("Library/Application Support/q-prompt"))
}

#[cfg(target_os = "linux")]
fn dirs_next_linux() -> Option<PathBuf> {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(|p| PathBuf::from(p).join("q-prompt"))
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|p| PathBuf::from(p).join(".local/share/q-prompt"))
        })
}
