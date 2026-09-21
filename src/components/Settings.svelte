<script>
  import { createEventDispatcher } from 'svelte';
  import { call } from '../lib/tauri.js';

  const dispatch = createEventDispatcher();
  export let settings;

  let form = { ...settings };
  let saving = false;

  const providers = [
    { id: 'deepseek', name: 'DeepSeek', base: 'https://api.deepseek.com/v1', model: 'deepseek-chat' },
    { id: 'ark', name: '火山方舟 Ark', base: 'https://ark.cn-beijing.volces.com/api/v3', model: 'doubao-seed-1-6-250615' },
    { id: 'ollama', name: '本地 Ollama', base: 'http://127.0.0.1:11434/v1', model: 'qwen2.5-coder:7b' },
    { id: 'custom', name: '自定义 OpenAI 兼容', base: '', model: '' },
  ];

  function pickProvider(id) {
    form.provider = id;
    const p = providers.find((x) => x.id === id);
    if (p && p.base) {
      form.api_base = p.base;
      form.chat_model = p.model;
    }
    if (id === 'ollama') form.api_key = '';
  }

  async function save() {
    saving = true;
    try {
      await call('save_settings', { settings: form });
      if (form.workspace_dir) {
        await call('set_workspace', { path: form.workspace_dir });
      }
      dispatch('saved', form);
    } catch (e) {
      dispatch('error', String(e.message || e));
    } finally {
      saving = false;
    }
  }
</script>

<div class="modal-mask" on:click={() => dispatch('close')}>
  <div class="modal" on:click|stopPropagation>
    <h3>模型与工作区设置</h3>

    <div class="form-row">
      <label>服务商</label>
      <select value={form.provider} on:change={(e) => pickProvider(e.target.value)}>
        {#each providers as p}
          <option value={p.id}>{p.name}</option>
        {/each}
      </select>
    </div>

    <div class="form-row">
      <label>API 地址</label>
      <input bind:value={form.api_base} placeholder="https://…/v1" />
    </div>

    <div class="form-row">
      <label>API Key</label>
      <input type="password" bind:value={form.api_key} placeholder="仅保存在本机 local-settings.json" />
    </div>

    <div class="form-row">
      <label>对话模型</label>
      <input bind:value={form.chat_model} />
    </div>

    <div class="form-row">
      <label>FIM 模型</label>
      <input bind:value={form.fim_model} placeholder="留空则回退对话模型（Ark 自动用对话接口模拟）" />
    </div>

    <div class="form-row">
      <label>超时(秒)</label>
      <input type="number" min="5" max="600" bind:value={form.timeout_secs} />
    </div>

    <div class="form-row">
      <label>代理</label>
      <input bind:value={form.proxy} placeholder="留空不走代理，如 http://127.0.0.1:7890" />
    </div>

    <div class="form-row">
      <label>工作目录</label>
      <input bind:value={form.workspace_dir} placeholder="例如 E:\Code\my-project（沙箱根目录）" />
    </div>

    <p class="warn">
      安全：Key 仅写入用户配置目录的 local-settings.json，已被 .gitignore 排除；文件读写被限制在工作目录内。
    </p>

    <div class="modal-actions">
      <button class="secondary" on:click={() => dispatch('close')}>取消</button>
      <button on:click={save} disabled={saving}>保存</button>
    </div>
  </div>
</div>

<style>
  .warn {
    color: var(--warn);
    font-size: 11px;
    line-height: 1.6;
    margin-top: 6px;
  }
</style>
