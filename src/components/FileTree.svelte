<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { call } from '../lib/tauri.js';
  import TreeNode from './TreeNode.svelte';

  const dispatch = createEventDispatcher();
  export let root = '';

  let roots = [];
  let error = '';

  async function refresh() {
    error = '';
    if (!root) {
      roots = [];
      return;
    }
    try {
      roots = await call('list_dir', { relPath: null });
    } catch (e) {
      error = String(e.message || e);
    }
  }

  onMount(refresh);
  $: if (root) refresh();
</script>

<div class="panel" style="border-right:1px solid var(--border)">
  <div class="panel-header">
    资源管理器
    <button class="secondary" style="float:right;padding:0 8px" on:click={refresh}>刷新</button>
  </div>
  <div class="tree-body">
    {#if !root}
      <div class="hint">请先在【设置】中打开工作目录</div>
    {:else if error}
      <div class="hint" style="color:var(--err)">{error}</div>
    {:else}
      {#each roots as node (node.rel_path)}
        <TreeNode {node} on:open={(e) => dispatch('open', e.detail)} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .tree-body {
    flex: 1;
    overflow: auto;
    padding: 4px 0;
  }
  .hint {
    color: var(--text-dim);
    padding: 10px;
    line-height: 1.6;
  }
</style>
