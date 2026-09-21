// release 下不附带控制台窗口，保持绿色单 exe 体验
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri_codex_ide_lib::run()
}
