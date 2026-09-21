<script>
  import { createEventDispatcher } from 'svelte';
  import { call } from '../lib/tauri.js';

  const dispatch = createEventDispatcher();
  export let settings;

  let form = { ...settings };
  let saving = false;

  // 全局模型注册表：每个供应商的可用模型（对话模型 + FIM 模型均从此取）
  // 集中在此维护，避免 UI 选项、默认值、路由判断分散散落
  const providerRegistry = {
    deepseek: {
      name: 'DeepSeek',
      base: 'https://api.deepseek.com/v1',
      defaultChat: 'deepseek-chat',
      defaultFim: 'deepseek-chat',
      chatModels: [
        { id: 'deepseek-chat', label: 'deepseek-chat（通用对话）' },
        { id: 'deepseek-reasoner', label: 'deepseek-reasoner（深度思考）' },
      ],
      fimModels: [
        { id: 'deepseek-chat', label: 'deepseek-chat（回退对话模型）' },
      ],
    },
    ark: {
      name: '火山方舟 Ark（按量）',
      base: 'https://ark.cn-beijing.volces.com/api/v3',
      defaultChat: 'doubao-seed-evolving',
      defaultFim: 'doubao-seed-evolving',
      // 实测该账号按量通道可用文本模型（2026-09 验证）
      chatModels: [
        { id: 'doubao-seed-evolving', label: 'doubao-seed-evolving（按量通道实测可用）' },
      ],
      // Ark 无原生 FIM，走 chat 模拟
      fimModels: [
        { id: 'doubao-seed-evolving', label: 'doubao-seed-evolving（用对话接口模拟）' },
      ],
    },
    'ark-agent-plan': {
      name: '火山方舟 Agent Plan（订阅）',
      base: 'https://ark.cn-beijing.volces.com/api/plan/v3',
      defaultChat: 'doubao-seed-evolving',
      defaultFim: 'deepseek-v4.1-flash',
      chatModels: [
        { id: 'doubao-seed-evolving', label: '🔥 doubao-seed-evolving｜多文件工程 Rust+JS（主力推荐）' },
        { id: 'deepseek-v4-pro', label: 'deepseek-v4-pro｜JS逆向底层逻辑（高消耗）' },
        { id: 'deepseek-v4.1-flash', label: '⚡ deepseek-v4.1-flash｜识图看报错性价比（5折）' },
        { id: 'deepseek-v4-flash', label: 'deepseek-v4-flash｜轻量脚本省额度' },
        { id: 'kimi-k2.8-preview', label: 'kimi-k2.8-preview｜百万上下文读大仓库' },
        { id: 'kimi-k2.7-code', label: 'kimi-k2.7-code｜稳定代码专用' },
        { id: 'minimax-m3', label: 'minimax-m3｜Agent工具调用代码' },
        { id: 'glm-5.3', label: 'glm-5.3｜长代码理解' },
        { id: 'glm-5.3-flash', label: 'glm-5.3-flash｜识图+代码（折扣）' },
        { id: 'doubao-seed-2.1-turbo', label: 'doubao-seed-2.1-turbo｜快速原型' },
        { id: 'doubao-seed-2.0-lite', label: 'doubao-seed-2.0-lite｜轻量版' },
        { id: 'doubao-seed-2.0-mini', label: 'doubao-seed-2.0-mini｜极简' },
        { id: 'ark-code-latest', label: 'ark-code-latest｜方舟自动路由' },
        { id: 'kimi-k3', label: 'kimi-k3｜长上下文（实测可调用）' },
      ],
      // Plan 接口无原生 FIM，走 chat 模拟
      fimModels: [
        { id: 'deepseek-v4.1-flash', label: '⚡ deepseek-v4.1-flash（用对话接口模拟·推荐）' },
        { id: 'doubao-seed-evolving', label: 'doubao-seed-evolving（用对话接口模拟）' },
        { id: 'deepseek-v4-flash', label: 'deepseek-v4-flash（用对话接口模拟）' },
        { id: 'kimi-k2.7-code', label: 'kimi-k2.7-code（用对话接口模拟）' },
      ],
    },
    'ark-coding-plan': {
      name: '火山方舟 Coding Plan（订阅）',
      // 实测端点：Coding Plan 走 /api/coding/v3（不是 /api/plan/v3）
      base: 'https://ark.cn-beijing.volces.com/api/coding/v3',
      defaultChat: 'doubao-seed-evolving',
      defaultFim: 'deepseek-v4-flash',
      // 实测可用（2026-09 验证）；deepseek-v4.1-flash 不支持 coding plan，已剔除
      chatModels: [
        { id: 'doubao-seed-evolving', label: '🔥 doubao-seed-evolving｜CodingPlan主力工程' },
        { id: 'deepseek-v4-flash', label: 'deepseek-v4-flash｜批量脚本自动化' },
        { id: 'deepseek-v4-pro', label: 'deepseek-v4-pro｜复杂重构' },
        { id: 'glm-5.3-flash', label: 'glm-5.3-flash｜识图+代码（折扣）' },
        { id: 'minimax-m3', label: 'minimax-m3｜Agent工具调用' },
        { id: 'kimi-k2.8-preview', label: 'kimi-k2.8-preview｜百万上下文' },
        { id: 'ark-code-latest', label: 'ark-code-latest｜方舟自动路由' },
      ],
      // Coding Plan 无原生 FIM，走 chat 模拟
      fimModels: [
        { id: 'deepseek-v4-flash', label: 'deepseek-v4-flash（用对话接口模拟·省额度）' },
        { id: 'doubao-seed-evolving', label: 'doubao-seed-evolving（用对话接口模拟）' },
      ],
    },
    minimax: {
      name: 'MiniMax 开放平台',
      base: 'https://api.minimax.cn/v1',
      defaultChat: 'MiniMax-M3',
      defaultFim: 'MiniMax-M2.7-highspeed',
      chatModels: [
        { id: 'MiniMax-M3', label: 'MiniMax-M3（1M 上下文 · 最新旗舰）' },
        { id: 'MiniMax-M2.7', label: 'MiniMax-M2.7（204K · 约 60 TPS）' },
        { id: 'MiniMax-M2.7-highspeed', label: 'MiniMax-M2.7-highspeed（204K · 约 100 TPS）' },
        { id: 'MiniMax-M2.5', label: 'MiniMax-M2.5（204K · 顶尖性能）' },
        { id: 'MiniMax-M2.5-highspeed', label: 'MiniMax-M2.5-highspeed（204K · 极速）' },
        { id: 'MiniMax-M2.1', label: 'MiniMax-M2.1（204K · 编程强化）' },
        { id: 'MiniMax-M2.1-highspeed', label: 'MiniMax-M2.1-highspeed（204K · 编程极速）' },
        { id: 'MiniMax-M2', label: 'MiniMax-M2（204K · 编码与 Agent）' },
      ],
      // MiniMax 无原生 FIM，走 chat 模拟
      fimModels: [
        { id: 'MiniMax-M2.7-highspeed', label: 'MiniMax-M2.7-highspeed（用对话接口模拟·推荐）' },
        { id: 'MiniMax-M3', label: 'MiniMax-M3（用对话接口模拟·长上下文）' },
        { id: 'MiniMax-M2.5-highspeed', label: 'MiniMax-M2.5-highspeed（用对话接口模拟）' },
        { id: 'MiniMax-M2.1-highspeed', label: 'MiniMax-M2.1-highspeed（用对话接口模拟）' },
      ],
    },
    ollama: {
      name: '本地 Ollama',
      base: 'http://127.0.0.1:11434/v1',
      defaultChat: 'qwen2.5-coder:7b',
      defaultFim: 'qwen2.5-coder:7b',
      chatModels: [
        { id: 'qwen2.5-coder:7b', label: 'qwen2.5-coder:7b（代码模型）' },
        { id: 'qwen2.5:7b', label: 'qwen2.5:7b（通用模型）' },
        { id: 'codellama:7b', label: 'codellama:7b（代码模型）' },
      ],
      fimModels: [
        { id: 'qwen2.5-coder:7b', label: 'qwen2.5-coder:7b（FIM 补全）' },
        { id: 'codellama:7b-code', label: 'codellama:7b-code（FIM 补全）' },
      ],
    },
    custom: {
      name: '自定义 OpenAI 兼容',
      base: '',
      defaultChat: '',
      defaultFim: '',
      chatModels: [],
      fimModels: [],
    },
  };

  const providers = Object.entries(providerRegistry).map(([id, p]) => ({
    id,
    name: p.name,
    base: p.base,
    model: p.defaultChat,
  }));

  $: currentProvider = providerRegistry[form.provider] || providerRegistry.custom;
  $: chatModelOptions = currentProvider.chatModels;
  $: fimModelOptions = currentProvider.fimModels;
  $: isCustom = form.provider === 'custom';
  $: needsFimSimulate = ['ark', 'ark-agent-plan', 'ark-coding-plan', 'minimax'].includes(form.provider);

  function pickProvider(id) {
    form.provider = id;
    const reg = providerRegistry[id];
    if (!reg) return;
    if (reg.base) {
      form.api_base = reg.base;
    }
    form.chat_model = reg.defaultChat;
    form.fim_model = reg.defaultFim;
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
      {#if isCustom}
        <input bind:value={form.chat_model} placeholder="输入模型名称，如 gpt-4o-mini" />
      {:else}
        <select bind:value={form.chat_model}>
          {#each chatModelOptions as m}
            <option value={m.id}>{m.label}</option>
          {/each}
        </select>
      {/if}
    </div>

    <div class="form-row">
      <label>FIM 模型</label>
      {#if isCustom}
        <input bind:value={form.fim_model} placeholder="留空则回退对话模型" />
      {:else}
        <select bind:value={form.fim_model}>
          <option value="">（留空，回退对话模型）</option>
          {#each fimModelOptions as m}
            <option value={m.id}>{m.label}</option>
          {/each}
        </select>
      {/if}
      {#if !isCustom && needsFimSimulate}
        <p class="hint">* 无原生 FIM 接口，自动用对话接口模拟补全</p>
      {/if}
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
  .hint {
    color: var(--muted);
    font-size: 11px;
    margin: 4px 0 0 0;
  }
</style>
