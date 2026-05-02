# Keybinder — Agent Cheat Sheet

macOS app for managing skhd keyboard shortcuts. Tauri v2 (Rust) + Svelte 5 (TypeScript).

## Commands

| Task               | Command                                       |
| ------------------ | --------------------------------------------- |
| Dev server         | `make dev` or `bun run tauri dev`             |
| Frontend tests     | `bun test` (Vitest, jsdom)                    |
| Rust tests         | `cd src-tauri && cargo test`                  |
| All tests          | `make test`                                   |
| **Full typecheck** | **`bun run check`** (NOT `bun run typecheck`) |
| Lint everything    | `make lint`                                   |
| Format everything  | `make format`                                 |
| CI replica         | `make ci` (lint → test → build)               |
| Build DMG          | `make build` or `bun run tauri build`         |
| Release            | `make release VERSION=x.y.z`                  |

### Why `bun run check` instead of `typecheck`

`bun run typecheck` only runs `tsc --noEmit` — it skips `.svelte` files. `bun run check` runs `svelte-kit sync && svelte-check`, which validates Svelte components too. This is what CI uses.

## Architecture

- **Frontend** (`src/`): SvelteKit in SPA mode with `adapter-static` (`ssr = false` in `+layout.ts`). Svelte 5 runes throughout: `$state`, `$derived`, `$props`, `$effect`. Use `{@render children()}` not `<slot>`, `onclick={}` not `on:click={}`.

- **Backend** (`src-tauri/src/`): Rust with Tauri v2. Entry point `lib.rs` registers all command handlers. Structured as:
  - `commands/` — Tauri invoke handlers (API surface)
  - `services/` — Business logic (file I/O, service manager, theme monitor, etc.)
  - `models/` — Data types shared across the Tauri bridge
  - `parser/` — PEST-based skhd config parser (`grammar.pest`)

- **Frontend services** (`src/services/`): TypeScript wrappers calling Tauri `invoke()`. Single source of truth for the IPC boundary. Type definitions in `src/types.ts` must mirror Rust models.

- **Tests**: Frontend in `src/__tests__/` (Vitest + testing-library). Rust integration tests in `src-tauri/tests/`. Test setup mocks `window.__TAURI__`.

## skhd Variant Support (skhd vs skhd.zig)

The app supports both original `koekeishiya/skhd` and `jackielii/skhd.zig` fork:

- **Variant Detection**: `services/variant_detector.rs` detects which variant is installed via launchd, Homebrew, PATH fingerprinting, or .app bundle
- **Settings**: `services/settings.rs` persists user preference (auto/original/zig) and computes `effective_variant()`
- **Service Manager**: `services/service_manager.rs` dispatches all operations based on effective variant:
  - Original: `launchctl bootstrap/bootout` with `com.koekeishiya.skhd.plist`, `brew services restart skhd`, `skhd --reload`
  - Zig: `skhd --start-service`, `--stop-service`, `--restart-service`, `--reload`, `--install-service`, `--uninstall-service`
- **Error Messages**: All service errors are prefixed with variant name: "skhd: ..." or "skhd.zig: ..."
- **Config Paths**: Original checks `~/.config/skhd/skhdrc` then `~/.skhdrc`. Zig adds XDG support: `$XDG_CONFIG_HOME/skhd/skhdrc` first.
  - Implementation: `utils/path.rs` has `get_config_path_for_variant()` with variant-specific search order
  - Error messages list all searched paths for better debugging
  - Command: `get_active_config_path()` returns `ActiveConfigPathInfo { path, variant, searched_paths }`

## Key Gotchas

- **macOS only** — uses `objc` crate for system theme detection, `rfd` for native file dialogs, `plist` for app discovery.
- Version must stay in sync across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Use `make bump VERSION=x.y.z` to handle this.
- Vite dev server uses **strict port 1420** (set in `vite.config.js`). If that port is busy, dev server fails.
- `frontendDist` in `tauri.conf.json` is `../build` (SvelteKit adapter-static output), not the Vite default `dist`.
- `src-tauri/src/lib.rs` has `#![allow(unexpected_cfgs)]` — needed because the `objc` crate triggers clippy warnings. Don't remove it.
- ESLint has `no-unused-vars` and `@typescript-eslint/no-unused-vars` set to `off` — Svelte 5 runes syntax (like `$:` labels) causes false positives. Also `no-self-assign` is off for Svelte reactivity patterns (`config = config`).
- CI and release workflows run on `macos-latest` only — you cannot test Linux/Windows builds locally or in CI.

## Release Flow

Pushing a `v*` tag triggers GitHub Actions (release.yml) which builds a universal DMG (Intel + Apple Silicon). Use `make release VERSION=x.y.z` to bump, commit, tag, and push in one command.
