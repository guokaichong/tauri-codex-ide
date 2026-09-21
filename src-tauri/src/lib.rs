//! TauriCodexIDE 后端入口：注册 command、加载插件、启动窗口。
//! 本地只做 GUI / 文件 IO / 命令执行 / API 转发，不做模型推理。

mod agent;
mod api_gateway;
mod file_manager;
mod settings;
mod terminal;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // 配置
            settings::get_settings,
            settings::save_settings,
            // 文件沙箱
            file_manager::set_workspace,
            file_manager::list_dir,
            file_manager::read_file,
            file_manager::write_file,
            // API 转发
            api_gateway::chat,
            api_gateway::fim_completion,
            // Agent
            agent::agent_plan,
            agent::agent_diff,
            agent::apply_diff,
            // 终端白名单
            terminal::run_command,
        ])
        .run(tauri::generate_context!())
        .expect("TauriCodexIDE 启动失败");
}
