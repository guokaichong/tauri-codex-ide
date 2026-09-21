// Tauri 桥接：Tauri 内走真实 invoke；浏览器 npm run dev 时走内存 mock，便于调 UI。
import { invoke as tauriInvoke } from '@tauri-apps/api/core';

export const isTauri = '__TAURI_INTERNALS__' in window;

export async function call(cmd, args = {}) {
  if (isTauri) {
    return tauriInvoke(cmd, args);
  }
  return mock(cmd, args);
}

// ---------------- 浏览器 mock（数据不落盘、不联网） ----------------
const memFiles = new Map();
let mockWorkspace = '';

function mockSettings() {
  try {
    return JSON.parse(localStorage.getItem('mock-settings') || '{}');
  } catch {
    return {};
  }
}

async function mock(cmd, args) {
  switch (cmd) {
    case 'get_settings':
      return {
        provider: 'deepseek',
        api_base: 'https://api.deepseek.com/v1',
        api_key: '',
        chat_model: 'deepseek-chat',
        fim_model: '',
        timeout_secs: 60,
        proxy: '',
        workspace_dir: mockWorkspace,
        ...mockSettings(),
      };
    case 'save_settings': {
      localStorage.setItem('mock-settings', JSON.stringify(args.settings));
      mockWorkspace = args.settings.workspace_dir || '';
      return null;
    }
    case 'set_workspace':
      mockWorkspace = args.path || 'mock-workspace';
      return { root: mockWorkspace };
    case 'list_dir':
      // mock 工作区给一个示例文件 + 目录，便于浏览器内走通读写链路
      return [
        { name: 'src', rel_path: 'src', is_dir: true, size: 0 },
        { name: 'example.rs', rel_path: 'example.rs', is_dir: false, size: 0 },
      ];
    case 'read_file':
      return (
        memFiles.get(args.relPath) ??
        '// 浏览器 mock 文件，保存后内容存于内存\nfn main() {}\n'
      );
    case 'write_file':
      memFiles.set(args.relPath, args.content);
      return null;
    case 'agent_plan':
      return [
        '【修改规划】（浏览器 mock，未联网）',
        '1. 待修改文件：example.rs',
        '2. 请在 Tauri 客户端中配置 API Key 后使用真实能力。',
      ].join('\n');
    case 'agent_diff':
      return [
        '```diff',
        '--- a/example.rs',
        '+++ b/example.rs',
        '@@ -1,2 +1,3 @@',
 ' // 浏览器 mock 文件，保存后内容存于内存',
        ' fn main() {}',
        '+// mock 新增行',
        '```',
      ].join('\n');
    case 'chat':
      return '（浏览器 mock 回复，请在 Tauri 客户端中连接国内云端 API）';
    case 'fim_completion':
      return '';
    case 'apply_diff':
      return [{ path: 'example.rs', hunks: 1 }];
    case 'run_command':
      void args.command;
      throw new Error('浏览器模式不支持执行终端命令');
    default:
      throw new Error(`未知命令: ${cmd}`);
  }
}
