//! API 转发层：统一走 OpenAI 兼容协议。
//! 支持：DeepSeek / 火山方舟（按量/AgentPlan/CodingPlan） / MiniMax / Ollama / 自定义
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
    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "API 未返回内容".to_string())?;

    // 部分供应商模型默认带思考标签（MiniMax / DeepSeek R1 / Kimi 等），统一剥掉，面板只保留最终回答
    Ok(if needs_thinking_strip(&s.provider, &s.chat_model) {
        strip_thinking(&content)
    } else {
        content
    })
}

/// 判断是否需要剥离思考标签：MiniMax 全系 + DeepSeek reasoner + Kimi 系列 + 带 reasoner/thinking 的模型名
fn needs_thinking_strip(provider: &str, model: &str) -> bool {
    if provider == "minimax" {
        return true;
    }
    let m = model.to_lowercase();
    m.contains("reasoner") || m.contains("r1") || m.contains("kimi") || m.contains("thinking")
}

/// 剥掉响应里的 `<think>…</think>` / `<thinking>…</thinking>` 思考片段。
/// 部分模型思考无法通过参数关闭，只能后处理过滤。
fn strip_thinking(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        // 同时匹配 <think 和 <thinking（取先出现的那个）
        let open_think = rest.find("<think");
        let open_thinking = rest.find("<thinking");
        let open = match (open_think, open_thinking) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let Some(open_pos) = open else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open_pos]);
        // 找对应的闭合标签
        let close_think = rest[open_pos..].find("</think");
        let close_thinking = rest[open_pos..].find("</thinking");
        let close = match (close_think, close_thinking) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        match close {
            Some(close_pos) => {
                let tail = &rest[open_pos + close_pos..];
                rest = tail.find('>').map(|gt| &tail[gt + 1..]).unwrap_or("");
            }
            // 未闭合说明思考被截断，剩余部分一并丢弃
            None => return out.trim().to_string(),
        }
    }
    out.trim().to_string()
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
    if matches!(
        s.provider.as_str(),
        "ark" | "ark-agent-plan" | "ark-coding-plan" | "minimax"
    ) {
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
