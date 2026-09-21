<script>
  import { onMount, onDestroy, beforeUpdate, createEventDispatcher } from 'svelte';
  import monaco, { langByExt } from '../lib/monaco_setup.js';
  import { call } from '../lib/tauri.js';

  const dispatch = createEventDispatcher();
  export let tabs = [];
  export let activePath = '';

  let host;
  let editor;
  const models = new Map();

  function langOf(path) {
    const ext = path.split('.').pop()?.toLowerCase() || '';
    return langByExt[ext] || 'plaintext';
  }

  onMount(() => {
    editor = monaco.editor.create(host, {
      theme: 'vs-dark',
      automaticLayout: true,
      fontSize: 13,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      tabSize: 4,
      renderWhitespace: 'boundary',
    });

    // 上报选中代码（AI 面板据此附带片段，发送仍需用户勾选确认）
    editor.onDidChangeCursorSelection(() => {
      const text = editor.getModel()?.getValueInRange(editor.getSelection()) || '';
      if (text.trim()) {
        dispatch('select', { path: activePath, text });
      }
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      dispatch('save');
    });

    // Ctrl+. 触发 FIM 行内补全
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period, async () => {
      const model = editor.getModel();
      if (!model) return;
      const pos = editor.getPosition();
      const prompt = model.getValueInRange({
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: pos.lineNumber,
        endColumn: pos.column,
      });
      const lineCount = model.getLineCount();
      const suffix = model.getValueInRange({
        startLineNumber: pos.lineNumber,
        startColumn: pos.column,
        endLineNumber: lineCount,
        endColumn: model.getLineMaxColumn(lineCount),
      });
      try {
        const out = await call('fim_completion', {
          prompt,
          suffix,
          language: langOf(activePath),
        });
        if (out) editor.executeEdits('fim', [{ range: monaco.Range.fromPositions(pos), text: out }]);
      } catch (e) {
        dispatch('error', String(e.message || e));
      }
    });
  });

  function ensureModel(tab) {
    if (models.has(tab.path)) return models.get(tab.path);
    const model = monaco.editor.createModel(tab.content, langOf(tab.path));
    model.onDidChangeContent(() => {
      tab.content = model.getValue();
      tab.dirty = true;
      tabs = tabs;
    });
    models.set(tab.path, model);
    return model;
  }

  // 切换标签：懒创建 model
  $: if (editor && activePath) {
    const tab = tabs.find((t) => t.path === activePath);
    if (tab) editor.setModel(ensureModel(tab));
  }

  export async function saveActive() {
    const tab = tabs.find((t) => t.path === activePath);
    if (!tab) return;
    await call('write_file', { relPath: tab.path, content: tab.content });
    tab.dirty = false;
    tabs = tabs;
  }

  // 外部加载新内容后刷新 model
  beforeUpdate(() => {
    for (const t of tabs) {
      const m = models.get(t.path);
      if (m && !t.dirty && m.getValue() !== t.content) {
        m.setValue(t.content);
      }
    }
  });

  onDestroy(() => {
    models.forEach((m) => m.dispose());
    editor?.dispose();
  });
</script>

<div class="editor-host" bind:this={host}></div>
