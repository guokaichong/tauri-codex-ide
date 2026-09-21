//! 本地文件树 / 读写，强制沙箱：只能访问 settings.workspace_dir 之内。

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::settings;

#[derive(Debug, Serialize)]
pub struct FileNode {
    pub name: String,
    /// 相对工作目录的路径（前端展示用）
    pub rel_path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceInfo {
    pub root: String,
}

/// 把相对路径解析为沙箱内绝对路径，并校验未越界（防 ../ 逃逸、符号链接逃逸）。
fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let root = std::fs::canonicalize(root).map_err(|e| format!("工作目录不可用: {e}"))?;
    let target = root.join(rel);
    // 文件可能尚不存在（新建场景），先校验词法路径，父目录必须在沙箱内
    let exists = target.exists();
    let checked = if exists {
        std::fs::canonicalize(&target).map_err(|e| format!("路径解析失败: {e}"))?
    } else {
        let parent = target
            .parent()
            .ok_or("非法路径")?
            .canonicalize()
            .map_err(|e| format!("父目录不可用: {e}"))?;
        parent.join(target.file_name().ok_or("非法文件名")?)
    };
    if !checked.starts_with(&root) {
        return Err("越权访问：路径超出工作目录沙箱".into());
    }
    Ok(checked)
}

fn workspace_root() -> Result<PathBuf, String> {
    let s = settings::load();
    if s.workspace_dir.trim().is_empty() {
        return Err("尚未设置工作目录，请先打开文件夹".into());
    }
    let root = PathBuf::from(&s.workspace_dir);
    if !root.is_dir() {
        return Err("工作目录不存在".into());
    }
    Ok(root)
}

#[tauri::command]
pub fn set_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    let canon = std::fs::canonicalize(&p).map_err(|e| e.to_string())?;
    let mut s = settings::load();
    s.workspace_dir = canon.to_string_lossy().to_string();
    settings::save(&s)?;
    Ok(WorkspaceInfo {
        root: s.workspace_dir,
    })
}

#[tauri::command]
pub fn list_dir(rel_path: Option<String>) -> Result<Vec<FileNode>, String> {
    let root = workspace_root()?;
    let rel = rel_path.unwrap_or_default();
    let dir = resolve(&root, &rel)?;
    if !dir.is_dir() {
        return Err("目标不是文件夹".into());
    }
    let mut nodes = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        // 不展示噪声目录
        if name == "node_modules" || name == "target" || name == ".git" || name == "dist" {
            continue;
        }
        if name.starts_with('.') && name != ".gitee" && name != ".github" {
            continue;
        }
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let meta = entry.metadata().ok();
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        nodes.push(FileNode {
            name,
            rel_path: child_rel,
            is_dir: ft.is_dir(),
            size: meta.map(|m| m.len()).unwrap_or(0),
        });
    }
    // 目录在前，文件在后，各自按名称排序
    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(nodes)
}

#[tauri::command]
pub fn read_file(rel_path: String) -> Result<String, String> {
    let root = workspace_root()?;
    let p = resolve(&root, &rel_path)?;
    std::fs::read_to_string(&p).map_err(|e| format!("读取失败: {e}"))
}

#[tauri::command]
pub fn write_file(rel_path: String, content: String) -> Result<(), String> {
    let root = workspace_root()?;
    let p = resolve(&root, &rel_path)?;
    std::fs::write(&p, content).map_err(|e| format!("写入失败: {e}"))
}
