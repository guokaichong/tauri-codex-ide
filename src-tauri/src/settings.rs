//! 本地配置持久化：API 地址 / Key / 模型 / 代理 / 工作目录。
//! 文件固定名为 local-settings.json，仅存于用户配置目录，禁止提交仓库。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// 供应商标记：deepseek | ark | ollama | custom
    pub provider: String,
    /// OpenAI 兼容基址，如 https://api.deepseek.com/v1
    pub api_base: String,
    /// API 密钥（Ollama 可为空）
    pub api_key: String,
    /// 对话模型名
    pub chat_model: String,
    /// FIM 补全模型名（无则回退 chat_model）
    pub fim_model: String,
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// HTTP/HTTPS 代理，空则不使用
    pub proxy: String,
    /// 沙箱工作目录
    pub workspace_dir: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "deepseek".into(),
            api_base: "https://api.deepseek.com/v1".into(),
            api_key: String::new(),
            chat_model: "deepseek-chat".into(),
            fim_model: String::new(),
            timeout_secs: 60,
            proxy: String::new(),
            workspace_dir: String::new(),
        }
    }
}

/// 配置文件路径：%APPDATA%/TauriCodexIDE/local-settings.json
pub fn settings_path() -> Result<PathBuf, String> {
    let dir = dirs::config_dir()
        .ok_or("无法定位系统配置目录")?
        .join("TauriCodexIDE");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    Ok(dir.join("local-settings.json"))
}

pub fn load() -> AppSettings {
    settings_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}

/// 供 file_manager 等模块在修改工作目录后落盘
pub fn save(s: &AppSettings) -> Result<(), String> {
    let p = settings_path()?;
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(p, json).map_err(|e| format!("写入配置失败: {e}"))
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    load()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    save(&settings)
}
