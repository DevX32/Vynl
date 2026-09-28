import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')

export function getAppVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(resolve(repoRoot, 'package.json'), 'utf8'))
    return String(pkg.version ?? '0.0.0')
  } catch {
    return '0.0.0'
  }
}

export const REPO_URL = 'https://github.com/DevX32/Vynl'
export const RELEASES_URL = `${REPO_URL}/releases/latest`
export const STORE_URL = 'https://github.com/DevX32/vynl-store'
export const ISSUES_URL = `${REPO_URL}/issues`
export const ISSUES_NEW_URL = `${REPO_URL}/issues/new`
