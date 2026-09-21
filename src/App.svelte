<script>
  import { onMount } from 'svelte';
  import { call } from './lib/tauri.js';
  import FileTree from './components/FileTree.svelte';
  import Editor from './components/Editor.svelte';
  import AiPanel from './components/AiPanel.svelte';
  import SettingsModal from './components/Settings.svelte';

  let settings = null;
  let showSettings = false;
  let tabs = [];
  let activePath = '';
  let selection = null;
  let editorRef;

  let toast = null;
  function notify(msg, kind = 'ok', ms = 3000) {
    toast = { msg, kind };
    setTimeout(() => (toast = null), ms);
  }

  onMount(async () => {
    settings = await call('get_settings');
  });

  async function openFile(relPath) {
    let tab = tabs.find((t) => t.path === relPath);
    if (!tab) {
      const content = await call('read_file', { relPath });
      tab = { path: relPath, content, dirty: false };
      tabs = [...tabs, tab];
    }
    activePath = relPath;
  }

  function closeTab(path, ev) {
    ev?.stopPropagation();
    tabs = tabs.filter((t) => t.path !== path);
    if (activePath === path) activePath = tabs.at(-1)?.path || '';
  }

  async function saveActive() {
    try {
      await editorRef?.saveActive();
      notify('已保存');
    } catch (e) {
      notify(String(e.message || e), 'err', 5000);
    }
  }

  function onSelect(e) {
    selection = e.detail;
  }

  function onError(e) {
    notify(String(e.detail?.message || e.detail), 'err', 6000);
  }

  function onSaved(e) {
    settings = e.detail;
    showSettings = false;
    notify('设置已保存');
  }

  // 一键推送触发 Gitee CI（白名单命令串行，执行前确认）
  let confirmPush = false;
  let pushing = false;
  async function doPush() {
    confirmPush = false;
    pushing = true;
    const steps = [
      'git add -A',
      'git commit -m "chore: trigger gitee ci build"',
      'git push',
    ];
    try {
      for (const command of steps) {
        const r = await call('run_command', { command });
        const output = `${r.stdout}${r.stderr}`;
        // git commit 在无改动时返回非 0，属可忽略情况
        if (r.exit_code !== 0 && !output.includes('nothing to commit')) {
          notify(`${command} 失败：${output.slice(-200)}`, 'err', 6000);
          return;
        }
      }
      notify('已推送，Gitee CI 将开始云端打包');
    } catch (e) {
      notify(String(e.message || e), 'err', 6000);
    } finally {
      pushing = false;
    }
  }
</script>

<div class="app-grid">
  <div class="toolbar">
    <span class="title">TauriCodexIDE</span>
    <button class="secondary" on:click={() => (showSettings = true)}>设置 / 打开目录</button>
    <button class="secondary" on:click={saveActive} disabled={!activePath}>保存 (Ctrl+S 暂用按钮)</button>
    <button class="secondary" on:click={() => (confirmPush = true)} disabled={pushing}>一键推送打包</button>
    <span class="spacer"></span>
    <span class="workspace-label" title={settings?.workspace_dir || ''}>
      {settings?.workspace_dir ? '📂 ' + settings.workspace_dir : '未打开工作目录'}
    </span>
  </div>

  <div class="main-grid">
    <FileTree root={settings?.workspace_dir || ''} on:open={(e) => openFile(e.detail)} />

    <div class="content">
      <div class="tabs">
        {#each tabs as tab (tab.path)}
          <div
            class="tab"
            class:active={tab.path === activePath}
            on:click={() => (activePath = tab.path)}
          >
            {tab.path.split('/').pop()}
            {#if tab.dirty}<span class="dirty">●</span>{/if}
            <span class="close" on:click={(e) => closeTab(tab.path, e)}>×</span>
          </div>
        {/each}
      </div>
      {#if activePath}
        <Editor
          bind:this={editorRef}
          {tabs}
          {activePath}
          on:select={onSelect}
          on:error={onError}
          on:save={saveActive}
        />
      {:else}
        <div class="empty-hint">
          从左侧打开文件；选中代码后在右侧向 AI 提问（Ctrl + . 触发行内补全）
        </div>
      {/if}
    </div>

    <AiPanel {selection} on:error={onError} on:files-changed={() => notify('文件已更新，可重新打开查看')} />
  </div>
</div>

{#if showSettings && settings}
  <SettingsModal {settings} on:close={() => (showSettings = false)} on:saved={onSaved} on:error={onError} />
{/if}

{#if confirmPush}
  <div class="modal-mask" on:click={() => (confirmPush = false)}>
    <div class="modal" on:click|stopPropagation>
      <h3>确认推送源码到 Gitee？</h3>
      <p>将依次执行：<code>git add -A</code>、<code>git commit</code>、<code>git push</code>，推送后触发云端 CI 打包。</p>
      <div class="modal-actions">
        <button class="secondary" on:click={() => (confirmPush = false)}>取消</button>
        <button on:click={doPush}>确认推送</button>
      </div>
    </div>
  </div>
{/if}

{#if toast}
  <div class="toast {toast.kind}">{toast.msg}</div>
{/if}
