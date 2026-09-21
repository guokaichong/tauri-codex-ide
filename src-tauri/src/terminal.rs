//! 终端命令执行：仅白名单程序，不经 shell 解释，前后端双重确认。
//! 白名单：git / cargo / npm / pip。

use serde::Serialize;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const WHITELIST: [&str; 4] = ["git", "cargo", "npm", "pip"];

/// 高危关键字，命中即拒绝（即使落在白名单命令的参数里）。
const BLOCKED: [&str; 10] = [
    "rm -rf",
    "rm -r",
    "diskpart",
    "format ",
    "shutdown",
    "reg delete",
    "del /f",
    "del /s",
    "rd /s",
    "remove-item",
];

const MAX_OUTPUT_CHARS: usize = 20_000;

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub program: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// 按空白切分参数，支持双引号包裹（含空格的路径）。
fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn validate(command: &str) -> Result<(String, Vec<String>), String> {
    let lower = command.to_lowercase();
    for pattern in BLOCKED {
        if lower.contains(pattern) {
            return Err(format!("高危命令已拦截: {pattern}"));
        }
    }
    // 不经 shell 执行，因此明确拒绝 shell 元字符，避免误以为可以串联命令
    for meta in ['|', '&', ';', '>', '<', '`', '$', '\n'] {
        if command.contains(meta) {
            return Err(format!("不允许的 shell 元字符 `{meta}`：本 IDE 不经 shell 执行命令"));
        }
    }

    let parts = split_args(command);
    let program = parts.first().ok_or("命令为空")?.clone();
    if program.contains('/') || program.contains('\\') {
        return Err("只接受白名单命令名，不允许指定程序路径".into());
    }
    let stem = program.to_lowercase();
    let stem = stem.strip_suffix(".exe").unwrap_or(&stem).to_string();
    if !WHITELIST.contains(&stem.as_str()) {
        return Err(format!(
            "命令不在白名单内（仅允许 git / cargo / npm / pip）: {program}"
        ));
    }
    Ok((stem, parts[1..].to_vec()))
}

/// Windows 下 npm / pip 多为 .cmd 或 .exe 包装脚本，按顺序探测可用者。
fn candidates(program: &str) -> Vec<String> {
    if cfg!(windows) {
        match program {
            "npm" => vec!["npm.cmd".into(), "npm.exe".into()],
            "pip" => vec!["pip.exe".into(), "pip.cmd".into()],
            other => vec![format!("{other}.exe"), other.into()],
        }
    } else {
        vec![program.to_string()]
    }
}

fn tail(s: &str) -> String {
    if s.chars().count() <= MAX_OUTPUT_CHARS {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let kept: String = chars[chars.len() - MAX_OUTPUT_CHARS..].iter().collect();
    format!("...(输出过长，仅保留最后 {MAX_OUTPUT_CHARS} 字符)\n{kept}")
}

/// 在工作目录内执行白名单命令。前端须先弹窗确认再调用。
#[tauri::command]
pub async fn run_command(command: String) -> Result<CommandResult, String> {
    let (program, args) = validate(&command)?;

    let settings = crate::settings::load();
    let cwd = settings.workspace_dir.trim().to_string();
    if cwd.is_empty() || !std::path::Path::new(&cwd).is_dir() {
        return Err("尚未设置工作目录，无法执行命令".into());
    }

    let mut last_err = String::new();
    for candidate in candidates(&program) {
        let mut cmd = tokio::process::Command::new(&candidate);
        cmd.args(&args).current_dir(&cwd).kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        match cmd.output().await {
            Ok(output) => {
                return Ok(CommandResult {
                    program: candidate,
                    args,
                    exit_code: output.status.code(),
                    stdout: tail(&String::from_utf8_lossy(&output.stdout)),
                    stderr: tail(&String::from_utf8_lossy(&output.stderr)),
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                last_err = e.to_string();
            }
            Err(e) => return Err(format!("执行失败: {e}")),
        }
    }
    Err(format!("未找到可执行程序 {program}（{last_err}）"))
}
