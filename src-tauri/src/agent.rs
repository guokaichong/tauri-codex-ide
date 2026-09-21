//! 内置 Agent：需求 → 【修改规划】→ unified diff → 本地落盘。
//! 只处理前端显式传入（用户已确认）的上下文片段，不主动读取项目其它文件、不上传整个项目。

use serde::{Deserialize, Serialize};

use crate::api_gateway::{self, ChatMessage};
use crate::file_manager;

/// 内置系统提示词随二进制打包，运行时不依赖外部文件（绿色单 exe）。
const SYSTEM_PROMPT: &str = include_str!("../../agent-system-prompt.txt");

/// 用户确认后才会上传的上下文片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    /// 相对工作目录的路径
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyReport {
    pub path: String,
    pub hunks: usize,
}

fn build_messages(instruction: &str, context: &[ContextFile]) -> Vec<ChatMessage> {
    let mut user = String::from(instruction);
    if !context.is_empty() {
        user.push_str("\n\n以下是用户确认上传的上下文片段：");
        for f in context {
            user.push_str(&format!("\n\n文件：{}\n```\n{}\n```", f.path, f.content));
        }
    }
    vec![
        ChatMessage {
            role: "system".into(),
            content: SYSTEM_PROMPT.into(),
        },
        ChatMessage {
            role: "user".into(),
            content: user,
        },
    ]
}

/// 第一步：只产出【修改规划】，等用户确认，不产出代码。
#[tauri::command]
pub async fn agent_plan(requirement: String, context: Vec<ContextFile>) -> Result<String, String> {
    let instruction = format!(
        "需求：{requirement}\n\n请只输出【修改规划】：\n1. 待修改文件清单（完整相对路径）\n2. 每个文件的改动要点\n不要输出代码。"
    );
    api_gateway::chat(build_messages(&instruction, &context)).await
}

/// 第二步：用户确认规划后，产出 unified diff。
#[tauri::command]
pub async fn agent_diff(
    requirement: String,
    plan: String,
    context: Vec<ContextFile>,
) -> Result<String, String> {
    let instruction = format!(
        "需求：{requirement}\n\n已确认的修改规划：\n{plan}\n\n\
         请输出 unified diff 代码变更：每个文件一个 ```diff 代码块，头部为 `--- a/<相对路径>` 与 `+++ b/<相对路径>`，\
         改动用 @@ 行号块给出，上下文行保持与原文完全一致（含缩进）。除 diff 代码块外不要输出解释文字。"
    );
    api_gateway::chat(build_messages(&instruction, &context)).await
}

/// 第三步：用户确认 diff 后写入本地文件（写入路径仍受工作目录沙箱约束）。
#[tauri::command]
pub fn apply_diff(diff_text: String) -> Result<Vec<ApplyReport>, String> {
    let patches = parse_patches(&diff_text)?;
    if patches.is_empty() {
        return Err("未找到可应用的 diff 代码块".into());
    }

    // 先全部校验通过再写盘，避免中途失败留下半成品
    let mut staged: Vec<(String, String, usize)> = Vec::new();
    for patch in patches {
        let hunks: Vec<Hunk> = patch
            .hunks
            .into_iter()
            .filter(|h| !h.lines.is_empty())
            .collect();
        if hunks.is_empty() {
            return Err(format!("{}: diff 块中没有有效改动", patch.path));
        }
        let current = file_manager::read_file(patch.path.clone())?;
        let updated = apply_hunks(&current, &hunks, &patch.path)?;
        staged.push((patch.path, updated, hunks.len()));
    }

    let mut reports = Vec::new();
    for (path, content, hunks) in staged {
        file_manager::write_file(path.clone(), content)?;
        reports.push(ApplyReport { path, hunks });
    }
    Ok(reports)
}

#[derive(Debug, Clone)]
struct Hunk {
    /// diff 中的旧文件起始行号（@@ -old_start,count 的 old_start）
    old_start: usize,
    /// (' ' | '-' | '+', 内容)
    lines: Vec<(char, String)>,
}

#[derive(Debug)]
struct FilePatch {
    path: String,
    hunks: Vec<Hunk>,
}

/// 规范化 diff 头里的路径：去掉 a/ b/ 前缀、行尾制表符与 ./ 前缀。
fn clean_path(raw: &str) -> String {
    let s = raw.trim();
    let s = s.split('\t').next().unwrap_or(s).trim();
    let s = s.strip_prefix("a/").or_else(|| s.strip_prefix("b/")).unwrap_or(s);
    s.trim_start_matches("./").to_string()
}

fn parse_hunk_start(line: &str) -> Result<usize, String> {
    let seg = line
        .split(' ')
        .find(|s| s.starts_with('-'))
        .ok_or_else(|| format!("无法解析 hunk 头: {line}"))?;
    let digits: String = seg.chars().skip(1).take_while(|c| c.is_ascii_digit()).collect();
    digits
        .parse::<usize>()
        .map_err(|_| format!("无法解析 hunk 起始行: {line}"))
}

/// 只解析 ``` 代码块内的 unified diff，代码块之外的说明文字一律忽略。
fn parse_patches(text: &str) -> Result<Vec<FilePatch>, String> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut patches: Vec<FilePatch> = Vec::new();
    let mut in_fence = false;
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let lineno = i + 1;

        // 代码块围栏必须顶格，避免把 diff 里的上下文行当成围栏
        if line.starts_with("```") {
            in_fence = !in_fence;
            i += 1;
            continue;
        }
        if !in_fence {
            i += 1;
            continue;
        }

        // 文件头必须成对出现（--- 紧跟 +++），否则视为 hunk 内的删除行
        if line.starts_with("--- ")
            && lines
                .get(i + 1)
                .map(|n| n.starts_with("+++ "))
                .unwrap_or(false)
        {
            patches.push(FilePatch {
                path: clean_path(&lines[i + 1][4..]),
                hunks: Vec::new(),
            });
            i += 2;
            continue;
        }

        if line.starts_with("@@") {
            let hunk = Hunk {
                old_start: parse_hunk_start(line).map_err(|e| format!("第 {lineno} 行: {e}"))?,
                lines: Vec::new(),
            };
            let patch = patches
                .last_mut()
                .ok_or_else(|| format!("第 {lineno} 行 diff 缺少 `--- ` 文件头"))?;
            patch.hunks.push(hunk);
            i += 1;
            continue;
        }

        if let Some(tag) = line.chars().next() {
            if matches!(tag, ' ' | '-' | '+') {
                if let Some(hunk) = patches.last_mut().and_then(|p| p.hunks.last_mut()) {
                    hunk.lines.push((tag, line[tag.len_utf8()..].to_string()));
                }
            }
        }
        i += 1;
    }

    patches.retain(|p| !p.path.is_empty() && !p.hunks.is_empty());
    Ok(patches)
}

fn apply_hunks(content: &str, hunks: &[Hunk], path: &str) -> Result<String, String> {
    let crlf = content.contains("\r\n");
    let src: Vec<String> = content
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let mut out: Vec<String> = Vec::with_capacity(src.len());
    let mut cursor = 0usize;

    for hunk in hunks {
        let start = hunk.old_start.saturating_sub(1);
        if start < cursor {
            return Err(format!("{path}: diff 块乱序或重叠，请重新生成变更"));
        }
        while cursor < start && cursor < src.len() {
            out.push(src[cursor].clone());
            cursor += 1;
        }
        for (tag, text) in &hunk.lines {
            match tag {
                ' ' => {
                    let actual = src
                        .get(cursor)
                        .ok_or_else(|| format!("{path}: 第 {} 行超出文件末尾", cursor + 1))?;
                    if actual != text {
                        return Err(format!(
                            "{path}: 第 {} 行上下文不一致，文件可能已被改动\n期望: {text}\n实际: {actual}",
                            cursor + 1
                        ));
                    }
                    out.push(actual.clone());
                    cursor += 1;
                }
                '-' => {
                    let actual = src
                        .get(cursor)
                        .ok_or_else(|| format!("{path}: 第 {} 行超出文件末尾", cursor + 1))?;
                    if actual != text {
                        return Err(format!(
                            "{path}: 第 {} 行待删除内容不一致，文件可能已被改动\n期望: {text}\n实际: {actual}",
                            cursor + 1
                        ));
                    }
                    cursor += 1;
                }
                '+' => out.push(text.clone()),
                _ => {}
            }
        }
    }
    while cursor < src.len() {
        out.push(src[cursor].clone());
        cursor += 1;
    }

    let sep = if crlf { "\r\n" } else { "\n" };
    Ok(out.join(sep))
}
