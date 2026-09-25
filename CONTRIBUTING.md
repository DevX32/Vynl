# Contributing to Vynl

Thank you for your interest in contributing to Vynl! This guide will help you get started.

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) (JavaScript runtime & package manager)
- [Tauri CLI](https://tauri.app/v2/guide/installation/) (`bun add -d @tauri-apps/cli`)
- [Git](https://git-scm.com/)

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
   git commit -m "test: add tests"
   ```

### Code Style

- **TypeScript/Svelte**: Strict mode enabled, no unused variables
- **Rust**: Follow standard Rust conventions, run `cargo clippy` and `cargo fmt`
- **Commit messages**: Use [Conventional Commits](https://www.conventionalcommits.org/)

### Running Tests

```bash
# Frontend unit tests
bun run test

# Frontend type checking
bun run typecheck

# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Rust linting
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

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
├── tests/              # Test files
│   ├── web/            # Frontend unit/integration tests
│   └── e2e/            # End-to-end tests
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
- Uses `rodio` for audio playback, `lofty` for metadata, `yt-dlp` for fetching

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