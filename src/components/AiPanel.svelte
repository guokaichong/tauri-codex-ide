<script>
  import { createEventDispatcher } from 'svelte';
  import { call } from '../lib/tauri.js';

  const dispatch = createEventDispatcher();
  export let selection = null; // { path, text }

  let tab = 'agent'; // agent | terminal
  let request = '';
  let plan = '';
  let diff = '';
  let applied = [];
  let busy = false;
  let includeSelection = true;

  // 对话消息
  let messages = [];

  function snippets() {
    // 安全边界：仅在用户勾选时附带选中片段，绝不附加整个项目
    if (includeSelection && selection?.text) {
      return [{ path: selection.path, content: selection.text }];
    }
    return [];
  }

  async function genPlan() {
    if (!request.trim()) return;
    busy = true;
    diff = '';
    applied = [];
    try {
      // 后端内置 agent-system-prompt.txt，前端只传需求与用户确认的片段
      plan = await call('agent_plan', {
        requirement: request,
        context: snippets(),
      });
    } catch (e) {
      dispatch('error', String(e.message || e));
    } finally {
      busy = false;
    }
  }

  async function genDiff() {
    busy = true;
    try {
      diff = await call('agent_diff', {
        requirement: request,
        plan,
        context: snippets(),
      });
    } catch (e) {
      dispatch('error', String(e.message || e));
    } finally {
      busy = false;
    }
  }

  let confirmApply = false;
  async function doApply() {
    confirmApply = false;
    busy = true;
    try {
      applied = await call('apply_diff', { diffText: diff });
      dispatch('files-changed');
    } catch (e) {
      dispatch('error', String(e.message || e));
    } finally {
      busy = false;
    }
  }

  async function ask() {
    const content = request.trim();
    if (!content) return;
    const msgs = [
      ...messages,
      { role: 'user', content: [content, snippetBlock()].filter(Boolean).join('\n\n') },
    ];
    messages = msgs;
    request = '';
    busy = true;
    try {
      const reply = await call('chat', { messages: msgs });
      messages = [...msgs, { role: 'assistant', content: reply }];
    } catch (e) {
      dispatch('error', String(e.message || e));
    } finally {
      busy = false;
    }
  }

  function snippetBlock() {
    const ss = snippets();
    if (!ss.length) return '';
    return `【用户确认上传的代码片段】文件：${ss[0].path}\n\`\`\`\n${ss[0].content}\n\`\`\``;
  }

  // ---------------- 终端 ----------------
  let cmdLine = '';
  let termOutput = '';
  let pendingCmd = null;

  // 后端按整条命令字符串校验白名单（不经 shell），此处只做空值拦截
  function tryRun() {
    if (!cmdLine.trim()) return;
    pendingCmd = cmdLine.trim();
  }

  async function doRun() {
    const command = pendingCmd;
    pendingCmd = null;
    busy = true;
    try {
      const r = await call('run_command', { command });
      termOutput += `$ ${r.program} ${r.args.join(' ')}\n${r.stdout}${r.stderr}\n[exit ${r.exit_code}]\n\n`;
    } catch (e) {
      termOutput += `$ ${command}\n[拒绝/错误] ${String(e.message || e)}\n\n`;
    } finally {
      busy = false;
    }
  }
</script>

<div class="panel ai-panel">
  <div class="panel-header">
    <div class="tabs-mini">
      <button class="mini" class:on={tab === 'agent'} on:click={() => (tab = 'agent')}>AI 助手</button>
      <button class="mini" class:on={tab === 'terminal'} on:click={() => (tab = 'terminal')}>终端</button>
    </div>
  </div>

  {#if tab === 'agent'}
    <div class="body">
      {#if selection?.text}
        <label class="snippet-opt">
          <input type="checkbox" bind:checked={includeSelection} />
          <span>附带选中片段上传（{selection.path}，{selection.text.length} 字符）</span>
        </label>
      {:else}
        <div class="no-sel">未选中文本；不会上传任何代码。</div>
      {/if}

      <div class="chat-log">
        {#each messages as m}
          <div class="msg {m.role}">
            <div class="role">{m.role === 'user' ? '我' : 'AI'}</div>
            <pre>{m.content}</pre>
          </div>
        {/each}
        {#if plan}
          <div class="msg assistant">
            <div class="role">修改规划</div>
            <pre>{plan}</pre>
          </div>
        {/if}
        {#if diff}
          <div class="msg assistant">
            <div class="role">待应用 Diff</div>
            <pre class="diff">{diff}</pre>
          </div>
        {/if}
        {#if applied.length}
          <div class="msg assistant">
            <div class="role">应用结果</div>
            <pre>{applied
              .map((f) => `✓ ${f.path}（应用 ${f.hunks} 个改动块）`)
              .join('\n')}</pre>
          </div>
        {/if}
      </div>

      <textarea
        rows="4"
        bind:value={request}
        placeholder="描述需求，例如：为 settings.rs 增加配置校验…"
      ></textarea>
      <div class="actions">
        <button on:click={ask} disabled={busy}>对话</button>
        <button class="secondary" on:click={genPlan} disabled={busy}>① 生成规划</button>
        <button class="secondary" on:click={genDiff} disabled={busy || !plan}>② 生成Diff</button>
        <button class="secondary" on:click={() => (confirmApply = true)} disabled={busy || !diff}>
          ③ 应用
        </button>
      </div>
      <div class="tip">流程：生成规划 → 人工审阅 → 生成 diff → 确认应用写入本地文件</div>
    </div>
  {:else}
    <div class="body">
      <pre class="term-output">{termOutput || '白名单命令：git / cargo / npm / pip（执行前需确认；高危参数拦截）'}</pre>
      <input bind:value={cmdLine} placeholder="例如：git status" on:keydown={(e) => e.key === 'Enter' && tryRun()} />
      <div class="actions">
        <button on:click={tryRun} disabled={busy || !cmdLine.trim()}>执行（需确认）</button>
      </div>
    </div>
  {/if}
</div>

{#if confirmApply}
  <div class="modal-mask" on:click={() => (confirmApply = false)}>
    <div class="modal" on:click|stopPropagation>
      <h3>确认应用代码变更？</h3>
      <p>以下 diff 将写入工作目录内的本地文件，此操作不可自动撤销：</p>
      <pre class="diff confirm-diff">{diff}</pre>
      <div class="modal-actions">
        <button class="secondary" on:click={() => (confirmApply = false)}>取消</button>
        <button on:click={doApply}>确认写入</button>
      </div>
    </div>
  </div>
{/if}

{#if pendingCmd}
  <div class="modal-mask" on:click={() => (pendingCmd = null)}>
    <div class="modal" on:click|stopPropagation>
      <h3>确认执行终端命令？</h3>
      <pre class="confirm-diff">$ {pendingCmd}</pre>
      <div class="modal-actions">
        <button class="secondary" on:click={() => (pendingCmd = null)}>取消</button>
        <button on:click={doRun}>确认执行</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .ai-panel {
    border-right: none;
    border-left: 1px solid var(--border);
  }
  .tabs-mini {
    display: flex;
    gap: 6px;
  }
  .mini {
    background: transparent;
    color: var(--text-dim);
    padding: 2px 10px;
  }
  .mini.on {
    background: var(--accent);
    color: #fff;
  }
  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 8px;
    gap: 6px;
    min-height: 0;
  }
  .snippet-opt,
  .no-sel {
    font-size: 11px;
    color: var(--warn);
    display: flex;
    gap: 6px;
    align-items: flex-start;
  }
  .no-sel {
    color: var(--text-dim);
  }
  .chat-log {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 6px;
    background: #1b1b1c;
    min-height: 120px;
  }
  .msg {
    margin-bottom: 8px;
  }
  .role {
    font-size: 10px;
    color: var(--text-dim);
  }
  pre {
    white-space: pre-wrap;
    word-break: break-word;
    font-family: Consolas, monospace;
    font-size: 11px;
  }
  .diff {
    color: #9cdcfe;
  }
  textarea {
    resize: none;
    font-family: Consolas, monospace;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .tip {
    font-size: 10px;
    color: var(--text-dim);
  }
  .term-output {
    flex: 1;
    background: #000;
    color: #cccccc;
    padding: 8px;
    border-radius: 3px;
    overflow: auto;
    min-height: 200px;
  }
  .confirm-diff {
    max-height: 300px;
    overflow: auto;
    background: #1b1b1c;
    padding: 8px;
    border-radius: 3px;
    margin: 10px 0;
  }
</style>
