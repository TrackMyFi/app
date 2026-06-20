# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run the full Tauri desktop app (starts Vite dev server + Rust backend)
npm run tauri dev

# Run frontend-only (no Rust backend; useful for UI-only work)
npm run dev

# Type-check + build frontend
npm run build

# Run frontend unit tests
npm test

# Run tests in watch mode
npm run test:watch

# Seed the dev database from current prod data (macOS only)
npm run seed:dev
```

To build the Rust backend independently: `cd src-tauri && cargo build`.

## Architecture

TrackMyFI is a **local-first Tauri desktop app** (Rust backend + Vue 3 frontend) for personal FIRE (Financial Independence, Retire Early) tracking. Data lives in a local libSQL/SQLite database; Turso cloud sync is optional.

### IPC layer

The frontend never touches the database directly. All persistence goes through Tauri's IPC:

```
Vue component → Pinia store → src/lib/api/*.ts (invoke wrappers) → Rust command → libSQL
```

- `src/lib/api/` — thin wrappers around `invoke<T>('command_name', args)`. One file per domain.
- `src-tauri/src/commands/` — Rust `#[tauri::command]` handlers, one file per domain, all registered in `src-tauri/src/lib.rs`.

### State management

`src/stores/` holds Pinia stores (Composition API style, no Options API). Stores own reactive state and call `src/lib/api/` functions, then re-fetch to stay in sync. No Vuex/Pinia actions with mutation tracking — stores just re-call list functions after mutations.

### Business logic

`src/lib/` contains pure TypeScript with no Vue/Tauri dependencies:

- `fire/` — FIRE metrics, net worth series, projections, coast FIRE, contribution rates, forecasting
- `contributions/` — IRS contribution limits, contribution breakdown by account type
- `budget/` — budget calculations
- `transactions/` — balance preview, totals
- `csv/` — CSV parsing and balance projection
- `paychecks/` — paycheck calculations
- `balances/` — balance recency logic

All `*.test.ts` files live alongside their source and run under Vitest + jsdom.

### Database & migrations

- Local DB: `trackmyfi.db` in macOS app data dir (`~/Library/Application Support/com.trackmyfi.desktop/`)
- **Debug builds** (`tauri dev`) always use a `dev/` subdirectory — completely isolated from release data at compile time. You can never accidentally corrupt prod data during development.
- Migrations are hand-rolled SQL files in `src-tauri/migrations/`, numbered `0001_*.sql` onward, applied by `src-tauri/src/migrations.rs` on startup.
- To add a migration: create the next numbered `.sql` file, then add a `Migration` entry in `migrations.rs`.

### Cloud sync

Sync config (`sync.json`) is stored outside the database in the app config dir. The Turso auth token is stored in the OS keychain (debug builds use a separate keychain entry `com.trackmyfi.desktop.dev`). The `Db` struct in `src-tauri/src/db.rs` decides at startup whether to open a local file or a remote libSQL replica.

### Frontend details

- **Router**: uses `createWebHashHistory` (required for Tauri webview compatibility — no real HTTP server)
- **UI library**: Nuxt UI v4 (configured with emerald primary / mist neutral palette and icon overrides in `vite.config.ts`)
- **Icons**: Phosphor icons via `i-ph-*` class names (Iconify)
- **Charts**: Unovis (`@unovis/vue`)
- **Dates**: Luxon for date math; `@internationalized/date` for calendar primitives
- **Dialogs**: `window.confirm`/`window.alert` are no-ops in Tauri — always use `@tauri-apps/plugin-dialog`'s async `confirm()` instead

### Pages

`/` Dashboard, `/accounts` account list, `/accounts/:id` account detail, `/transactions`, `/paychecks`, `/contributions`, `/budget`, `/forecast`, `/settings`, `/onboarding`
