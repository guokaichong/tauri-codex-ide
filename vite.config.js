import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// 固定端口与 tauri.conf.json 的 devUrl 对齐
export default defineConfig({
  plugins: [
    svelte({
      // 桌面端 WebView 应用，a11y 点击告警不影响功能
      onwarn(warning, handler) {
        if (warning.code?.startsWith('a11y-')) return;
        handler?.(warning);
      },
    }),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: 'es2021',
    minify: 'esbuild',
    sourcemap: false,
  },
});
