# Vynl

A Minimal Music Player built with Tauri, Svelte, and Rust.

[![Support on Ko-fi](https://img.shields.io/badge/Support-Ko--fi-ff5e5b?logo=ko-fi&logoColor=white)](https://ko-fi.com/devx32)

### Player

- Built-in audio player with play/pause/seek/volume
- Queue management with reorder and remove
- Fullscreen player mode
- Persistent "Now Playing" state

### Library & Playlists

- Library scanning with incremental cache updates
- Create, rename, delete playlists
- Add/remove/reorder tracks
- Custom playlist cover art

### Lyrics

- Synced (LRC) and plain text lyrics
- Lyrics from LRCLIB (synced LRC and plain text)
- Lyrics embedding into audio files
- Lyrics overlay in the player

### Integration & UX

- Discord Rich Presence
- System tray
- Auto-updater
- Launch at startup
- Accent color theming
- i18n support

### Plugins

- Plugin runtime (JavaScript/TypeScript) with `onLoad`/`onEnable`/`onDisable`/`onUnload` lifecycle hooks
- Built-in plugin store: search, categories, one-click install, auto-updates
- Providers for lyrics, artist info, and collection resolution
- Plugin-contributed settings sections, Home page sections, and full pages
- Player observation + shared playback control (gated by a `player` permission)
- Dev folder installs with live reload, plus `.zip` sideloading
- **Listen Along** real-time synchronized listening sessions (host a session, share a code, friends follow your playback) ships as a store plugin in [vynl-store](https://github.com/DevX32/vynl-store/tree/main/plugins/listen-along)

## Platform Support

| Platform | Status |
| -------- | ------ |
| Windows  | Supported |
| Linux    | Supported |
| Android  | Supported — Companion App |
| macOS    | Not available |

## Development

```bash
# Install dependencies
bun install

# Start dev server
bun run tauri:dev
```

### Prerequisites

- [Rust](https://rustup.rs/) (via `rustup`)
- [Bun](https://bun.sh/) (or npm) — installs the local `@tauri-apps/cli` automatically

## Scripts

| Command | Description |
| ------- | ----------- |
| `bun run dev` | Vite dev server (frontend only) |
| `bun run tauri:dev` | Full Tauri app with hot reload |
| `bun run build` | Build frontend to `web/dist/` |
| `bun run tauri:build` | Build and package for current platform |
| `bun run typecheck` | TypeScript + Svelte checks |

## Support

Vynl is free and open source, and development happens in spare time. If the app is useful to you, you can support it here:

[![Support on Ko-fi](https://img.shields.io/badge/Support-Ko--fi-ff5e5b?logo=ko-fi&logoColor=white)](https://ko-fi.com/devx32)

Every contribution goes back into maintenance, fixes, and new features.

## Caution

How you use the app is entirely up to you, and you are responsible for your own actions — including any content you download and any legal consequences that may follow. The Vynl maintainers take no responsibility for misuse, and the software is provided "as is", without warranty of any kind.

If you enjoy a track, buy it. Purchasing directly from the artists is the best way to support them and keep their work going.

## License
Copyright © 2026 Vynl Contributors

Vynl is a player and library manager for audio you already have the right to listen to. It does not provide, host, or distribute any music itself, and it does not endorse or encourage obtaining copyrighted material without permission. Copyright law varies by country, so make sure you understand the rules that apply to you before using any tool that can save audio to your device.

GPL-3.0. See [LICENSE](LICENSE).
