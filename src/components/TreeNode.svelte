<script>
  // 显式自导入：递归渲染目录树（规避 dev 模式下隐式自引用未定义）
  import TreeNode from './TreeNode.svelte';
  import { createEventDispatcher } from 'svelte';
  import { call } from '../lib/tauri.js';

  const dispatch = createEventDispatcher();
  export let node;

  let open = false;
  let children = [];
  let loaded = false;
  let loading = false;

  async function toggle() {
    if (!node.is_dir) {
      dispatch('open', node.rel_path);
      return;
    }
    open = !open;
    if (open && !loaded) {
      loading = true;
      try {
        children = await call('list_dir', { relPath: node.rel_path });
        loaded = true;
      } finally {
        loading = false;
      }
    }
  }
</script>

<div class="node" class:open>
  <div class="row" on:click={toggle} title={node.rel_path}>
    {#if node.is_dir}
      <span class="arrow">{open ? '▾' : '▸'}</span>
      <span class="ico">📁</span>
    {:else}
      <span class="arrow"></span>
      <span class="ico">📄</span>
    {/if}
    <span class="name">{node.name}</span>
  </div>
  {#if open && node.is_dir}
    <div class="children">
      {#if loading}<div class="loading">加载中…</div>{/if}
      {#each children as child (child.rel_path)}
        <TreeNode node={child} on:open={(e) => dispatch('open', e.detail)} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
  }
  .row:hover {
    background: #2a2d2e;
  }
  .arrow {
    width: 12px;
    color: var(--text-dim);
    font-size: 10px;
  }
  .ico {
    font-size: 12px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .children {
    padding-left: 14px;
  }
  .loading {
    padding: 2px 24px;
    color: var(--text-dim);
  }
</style>
