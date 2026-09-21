// 精简 Monaco：仅注册需要的 basic-languages，静态资源全部本地打包（无 CDN）。
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';

import 'monaco-editor/esm/vs/basic-languages/rust/rust.contribution';
import 'monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution';
import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution';
import 'monaco-editor/esm/vs/basic-languages/python/python.contribution';
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution';
import 'monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution';
import 'monaco-editor/esm/vs/basic-languages/css/css.contribution';
import 'monaco-editor/esm/vs/basic-languages/html/html.contribution';
import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution';
import 'monaco-editor/esm/vs/basic-languages/go/go.contribution';
// JSON 没有 Monarch 词法（basic-languages 下不存在），走自带语言服务
import 'monaco-editor/esm/vs/language/json/monaco.contribution';

import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import JsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';

self.MonacoEnvironment = {
  getWorker(_moduleId, label) {
    return label === 'json' ? new JsonWorker() : new EditorWorker();
  },
};

export const langByExt = {
  rs: 'rust',
  js: 'javascript',
  mjs: 'javascript',
  ts: 'typescript',
  json: 'json',
  py: 'python',
  yml: 'yaml',
  yaml: 'yaml',
  md: 'markdown',
  css: 'css',
  html: 'html',
  sh: 'shell',
  go: 'go',
  toml: 'ini',
  txt: 'plaintext',
};

export default monaco;
