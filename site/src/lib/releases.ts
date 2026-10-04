import { request } from 'node:https'
import { REPO_URL } from './repo'

export interface Asset {
  label: string
  file: string
  url: string
  size: string
}

export interface LinuxAsset extends Asset {
  format: string
}

export interface AndroidAsset extends Asset {
  abi: string
  recommended: boolean
}

export interface LatestRelease {
  version: string
  publishedAt: string | null
  windows: Asset | null
  linux: LinuxAsset[]
  android: AndroidAsset[]
}

const API_URL = 'https://api.github.com/repos/DevX32/Vynl/releases/latest'
const TIMEOUT_MS = 8000
const MB = 1024 * 1024

interface RawRelease {
  tag_name?: string
  published_at?: string
  assets?: { name: string; browser_download_url: string; size: number }[]
}

function getJson(url: string, headers: Record<string, string>): Promise<RawRelease> {
  return new Promise((resolve, reject) => {
    const req = request(url, { method: 'GET', headers, agent: false }, (res) => {
      const chunks: Buffer[] = []
      res.on('data', (chunk: Buffer) => chunks.push(chunk))
      res.on('end', () => {
        res.destroy()
        if (!res.statusCode || res.statusCode >= 400) {
          reject(new Error(`GitHub API responded ${res.statusCode}`))
          return
        }
        try {
          resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')))
        } catch (error) {
          reject(error)
        }
      })
    })

    req.setTimeout(TIMEOUT_MS, () => req.destroy(new Error('GitHub API request timed out')))
    req.on('error', reject)
    req.end()
  })
}

function formatSize(bytes: number): string {
  return bytes >= MB ? `${(bytes / MB).toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB`
}

function stripVersionPrefix(file: string): string {
  return file.replace(/^Vynl[_-]v?[\d.]+[_-]/, '')
}

const SUPPORTED_ABIS = ['arm64-v8a', 'armeabi-v7a', 'x86_64'] as const

export const RECOMMENDED_ABI: string = 'arm64-v8a'

function parseApkAbi(file: string): string | null {
  if (!/^Vynl-mobile-.*\.apk$/.test(file)) return null
  for (const abi of SUPPORTED_ABIS) {
    if (file.endsWith(`-${abi}.apk`)) return abi
  }
  return null
}

export async function getLatestRelease(): Promise<LatestRelease | null> {
  if (process.env.VYNL_OFFLINE_BUILD) return null

  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN

  try {
    const data = await getJson(API_URL, {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'vynl-site',
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    })

    const assets = data.assets ?? []
    if (!assets.length) return null

    const toAsset = (asset: (typeof assets)[number]): Asset => ({
      label: stripVersionPrefix(asset.name),
      file: asset.name,
      url: asset.browser_download_url,
      size: formatSize(asset.size)
    })

    const windowsAsset =
      assets.find((a) => a.name.endsWith('.exe') && !a.name.endsWith('.exe.sig')) ??
      assets.find((a) => a.name.endsWith('.msi'))

    const linux = assets
      .filter((a) => /\.(deb|rpm|AppImage)$/.test(a.name))
      .map<LinuxAsset>((a) => ({
        ...toAsset(a),
        format: a.name.endsWith('.AppImage') ? 'AppImage' : a.name.split('.').pop()!.toUpperCase()
      }))

    const android = assets
      .filter((a) => a.name.endsWith('.apk'))
      .map<AndroidAsset>((a) => ({
        ...toAsset(a),
        abi: parseApkAbi(a.name) ?? 'unknown',
        recommended: parseApkAbi(a.name) === RECOMMENDED_ABI
      }))
      .sort((a, b) => Number(b.recommended) - Number(a.recommended) || a.abi.localeCompare(b.abi))

    return {
      version: (data.tag_name ?? '').replace(/^v/, '') || 'latest',
      publishedAt: data.published_at ?? null,
      windows: windowsAsset ? toAsset(windowsAsset) : null,
      linux,
      android
    }
  } catch {
    return null
  }
}

export const FALLBACK_DOWNLOAD_URL = `${REPO_URL}/releases/latest`
