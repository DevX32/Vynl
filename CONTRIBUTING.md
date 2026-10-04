# Contributing to Vynl

Thank you for your interest in contributing to Vynl! This guide will help you get started.

## Prerequisites

- [Rust](https://rustup.rs/) 1.85 or newer — the backend uses edition 2024
- [Bun](https://bun.sh/) — JavaScript runtime and package manager
- [Git](https://git-scm.com/)
- A C compiler and CMake — see below

The [Tauri CLI](https://tauri.app/v2/guide/installation/) is already a dev
dependency, so `bun install` in the next step is all that is needed to get it.
Do not install it separately; doing so edits `package.json`.

### C toolchain

The audio pipeline compiles C libraries from source, so a C compiler and CMake
are required. `mp3lame-sys` builds LAME with `cc`, and `opusic-sys` builds libopus
with CMake.

| Platform | Install |
| --- | --- |
| Windows | Visual Studio 2022 with the "Desktop development with C++" workload (CMake ships with it) |
| Debian/Ubuntu | `sudo apt install build-essential cmake` |
| macOS | `xcode-select --install` plus `brew install cmake` |

On Windows, verify with `cmake --version`; the Visual Studio copy is not always on
`PATH`. Without these, `cargo build` fails while compiling the audio dependencies.

## Getting Started

### 1. Clone the Repository

```bash
git clone https://github.com/devx32/Vynl.git
cd Vynl
```

### 2. Install Dependencies

```bash
bun install
```

### 3. Run the Development Server

```bash
bun run tauri:dev
```

This starts the Tauri app with hot reload enabled.

For frontend-only development (without the Rust backend):

```bash
bun run dev
```

## Development Workflow

### Making Changes

1. Create a new branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. Make your changes following the code style guidelines below.

3. Test your changes:
   ```bash
   bun run typecheck    # TypeScript + Svelte type checking
   bun run build        # Build the frontend
   ```

4. Commit your changes using conventional commit format:
   ```bash
   git commit -m "feat: add new feature"
   git commit -m "fix: resolve issue"
   git commit -m "docs: update documentation"
   git commit -m "refactor: improve code structure"
   ```

### Code Style

- **TypeScript/Svelte**: Strict mode enabled, no unused variables
- **Rust**: Follow standard Rust conventions, run `cargo clippy` and `cargo fmt`
- **Commit messages**: Use [Conventional Commits](https://www.conventionalcommits.org/)

### Checks

There is currently no automated test suite, so these checks are what CI runs:

```bash
# Frontend type checking
bun run typecheck

# Rust linting and formatting
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Changes to the audio pipeline (`src-tauri/src/services/audio.rs`) should be
verified by hand as well: download a track and confirm the output sample rate,
duration, and tags.

### Building for Production

```bash
# Build for current platform
bun run tauri:build

# Or use the Tauri CLI directly
bun run tauri build
```

## Project Structure

```
Vynl/
├── src-tauri/          # Rust backend (Tauri)
│   ├── src/
│   │   ├── commands/   # Tauri command handlers
│   │   └── services/   # Core business logic
│   └── tauri.conf.json # Tauri configuration
├── web/                # Svelte 5 frontend
│   ├── src/
│   │   ├── components/ # Reusable UI components
│   │   ├── features/   # Feature modules
│   │   ├── lib/        # Shared utilities
│   │   └── state/      # Svelte stores/state
│   └── vite.config.ts  # Vite configuration
├── .github/workflows/  # CI/CD workflows
└── CONTRIBUTING.md     # This file
```

## Architecture Overview

### Frontend (Svelte 5)
- Uses Svelte 5 runes (`$state`, `$derived`, `$effect`) for reactivity
- Components are organized by feature
- State management uses `.svelte.ts` files with rune-based stores
- Vite handles bundling with code splitting for vendor libraries

### Backend (Rust/Tauri)
- Tauri 2 provides the desktop app shell
- Commands are exposed to the frontend via `invoke`
- Services handle: audio playback, library scanning, downloading, lyrics, etc.
- Uses `rodio` for audio playback, `lofty` for metadata, and `yt-dlp` for fetching
- Decodes and encodes audio entirely in-process: `symphonia` handles container
  parsing and AAC, `symphonia-adapter-libopus` supplies libopus for Opus streams,
  and `mp3lame-encoder` (LAME) produces the mp3 output

### Real-time Sync (Listen Along plugin)

Listen Along now lives in the [vynl-store](https://github.com/DevX32/vynl-store)
repo as a store plugin (`plugins/listen-along/`) and is developed there — it
uses the plugin API (its own page, `api.Player` shared playback, lyrics
provider) rather than app internals. Changes to the feature go to that repo,
not here; the app only provides the host APIs it builds on.

## Pull Requests

1. Ensure all tests pass
2. Update documentation if needed
3. Write a clear PR description explaining:
   - What the change does
   - Why it's needed
   - Any breaking changes
4. Link to any related issues

## Questions?

Feel free to open an issue for questions or discussion before starting work on larger features.
