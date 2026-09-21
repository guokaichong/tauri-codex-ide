//! API 转发层：统一走 OpenAI 兼容协议（DeepSeek / 火山 Ark / Ollama /v1）。
//! 密钥仅从本地配置读取，不落日志、不进仓库。

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::settings::{self, AppSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResp {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct FimChoice {
    text: String,
}

#[derive(Debug, Deserialize)]
struct FimResp {
    choices: Vec<FimChoice>,
}

fn build_client(s: &AppSettings) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(s.timeout_secs.max(5)))
        .user_agent("TauriCodexIDE/0.1");
    if !s.proxy.trim().is_empty() {
        let proxy = reqwest::Proxy::all(s.proxy.trim()).map_err(|e| format!("代理配置错误: {e}"))?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| format!("HTTP 客户端初始化失败: {e}"))
}

fn url(base: &str, tail: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), tail)
}

/// 对话补全。messages 由前端组装；调用方负责保证只含用户确认的片段。
#[tauri::command]
pub async fn chat(messages: Vec<ChatMessage>) -> Result<String, String> {
    let s = settings::load();
    if s.api_base.trim().is_empty() {
        return Err("未配置 API 地址".into());
    }
    if s.provider != "ollama" && s.api_key.trim().is_empty() {
        return Err("未配置 API Key".into());
    }

    let body = serde_json::json!({
        "model": s.chat_model,
        "messages": messages,
        "stream": false,
        "temperature": 0.2
    });

    let mut req = build_client(&s)?
        .post(url(&s.api_base, "/chat/completions"))
        .json(&body);
    if !s.api_key.trim().is_empty() {
        req = req.bearer_auth(s.api_key.trim());
    }

    let resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let raw = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("API 返回 {status}: {}", truncate(&raw, 500)));
    }
    let parsed: ChatResp =
        serde_json::from_str(&raw).map_err(|e| format!("响应解析失败: {e}; 原始: {}", truncate(&raw, 300)))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "API 未返回内容".into())
}

/// FIM 行内补全。language 用于给模型提示，suffix 为光标后文本。
#[tauri::command]
pub async fn fim_completion(prompt: String, suffix: String, language: String) -> Result<String, String> {
    let s = settings::load();
    let model = if s.fim_model.trim().is_empty() {
        s.chat_model.clone()
    } else {
        s.fim_model.clone()
    };
    let lang_hint = if language.is_empty() { String::new() } else { format!("# language: {language}\n") };

    // 不支持原生 /completions 的供应商标记：用 chat 接口模拟
    if s.provider == "ark" {
        let msg = ChatMessage {
            role: "user".into(),
            content: format!(
                "{lang_hint}仅输出补全代码，不要解释。\n已有代码：\n{prompt}\n<CURSOR/>\n{suffix}"
            ),
        };
        return chat(vec![msg]).await;
    }

    let body = serde_json::json!({
        "model": model,
        "prompt": format!("{lang_hint}{prompt}"),
        "suffix": suffix,
        "max_tokens": 128,
        "temperature": 0.1,
        "stream": false
    });
    let mut req = build_client(&s)?
        .post(url(&s.api_base, "/completions"))
        .json(&body);
    if !s.api_key.trim().is_empty() {
        req = req.bearer_auth(s.api_key.trim());
    }
    let resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let raw = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("API 返回 {status}: {}", truncate(&raw, 500)));
    }
    let parsed: FimResp =
        serde_json::from_str(&raw).map_err(|e| format!("响应解析失败: {e}; 原始: {}", truncate(&raw, 300)))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.text)
        .ok_or_else(|| "API 未返回补全".into())
}

fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
