import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { resolve } from 'path'

export default defineConfig({
  root: 'web',
  envDir: '..',
  plugins: [svelte()],
  resolve: {
    alias: {
      '@': resolve('./web/src'),
      '@lib': resolve('./web/src/lib'),
      '@components': resolve('./web/src/components'),
      '@features': resolve('./web/src/features'),
      '@state': resolve('./web/src/state'),
      '@assets': resolve('./web/src/assets')
    }
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    cssMinify: 'lightningcss',
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      input: {
        index: 'web/index.html'
      },
      output: {
        manualChunks: {
          'vendor-tauri': ['@tauri-apps/api'],
          'vendor-icons': ['lucide-svelte']
        }
      }
    }
  },
  server: {
    port: 5173,
    strictPort: true
  }
})
