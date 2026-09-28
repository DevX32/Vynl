import { cpSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..', '..')
const publicDir = resolve(here, '..', 'public')

const jobs = [
  { from: resolve(repoRoot, 'docs/screenshots'), to: resolve(publicDir, 'screenshots') },
  { from: resolve(repoRoot, 'src-tauri/icons/icon.png'), to: resolve(publicDir, 'app-icon.png') },
  { from: resolve(repoRoot, 'web/src/assets/fonts/Geist-Variable.woff2'), to: resolve(publicDir, 'fonts/Geist-Variable.woff2') },
  { from: resolve(repoRoot, 'web/src/assets/fonts/GeistMono-Variable.woff2'), to: resolve(publicDir, 'fonts/GeistMono-Variable.woff2') }
]

for (const { from, to } of jobs) {
  if (!existsSync(from)) {
    console.warn(`[sync-assets] skipped, not found: ${from}`)
    continue
  }
  mkdirSync(dirname(to), { recursive: true })
  cpSync(from, to, { recursive: true })
  console.log(`[sync-assets] ${from} -> ${to}`)
}
