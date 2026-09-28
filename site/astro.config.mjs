import { defineConfig } from 'astro/config'

export default defineConfig({
  site: 'https://devx32.github.io',
  base: '/Vynl',
  trailingSlash: 'ignore',
  outDir: './dist',
  build: {
    format: 'directory',
    inlineStylesheets: 'auto'
  },
  dev: {
    port: 4321,
    open: false
  }
})
