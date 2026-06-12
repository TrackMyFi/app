# TrackMyFI Phase 1 Implementation Plan (Core FIRE Loop)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the TrackMyFI desktop app from an empty repo and ship the Core FIRE Loop — configure a FIRE profile, track accounts and balance snapshots, and see live FIRE metrics on a dashboard.

**Architecture:** Tauri 2 desktop shell. Vue 3 frontend (all UI + FIRE math in TypeScript) talks to a Rust core over typed per-entity `invoke()` commands. Rust owns a local libSQL database file and runs migrations on startup. `ts-rs` generates TS types from Rust serde structs to keep the boundary in sync. Encryption + Turso cloud sync are deferred to a post-Phase-1 slice.

**Tech Stack:** Tauri 2.x · Vue 3 · Vite · NuxtUI (`@nuxt/ui`) · Vue Router · Pinia · libSQL (`libsql` crate, local mode) · `ts-rs` · Luxon · unovis (`@unovis/vue`) · Vitest

**Spec:** [`docs/superpowers/specs/2026-06-12-trackmyfi-phase-1-design.md`](../specs/2026-06-12-trackmyfi-phase-1-design.md)

---

## File Structure

**Rust core (`src-tauri/`):**
- `src/main.rs` / `src/lib.rs` — Tauri entrypoint, builder, state management, command registration
- `src/db.rs` — open local libSQL file, connection accessor, run migrations on startup
- `src/migrations.rs` — migration runner + ordered embedded SQL (or crate-based if spike succeeds)
- `migrations/0001_init.sql` — first migration (3 tables)
- `src/models.rs` — serde + `ts-rs` structs mirroring rows (shared across commands)
- `src/commands/mod.rs` — re-exports
- `src/commands/fire_profile.rs` — `get_fire_profile`, `upsert_fire_profile`
- `src/commands/accounts.rs` — account + balance commands
- `tests/roundtrip.rs` — integration smoke test (temp DB, migrate, round-trip each entity)

**Vue frontend (`src/`):**
- `lib/types/` — generated TS types from `ts-rs` (do not hand-edit)
- `lib/api/fireProfile.ts`, `lib/api/accounts.ts` — typed `invoke()` wrappers
- `lib/fire/types.ts` — pure input types for FIRE math
- `lib/fire/*.ts` — one file per pure calculation; `lib/fire/index.ts` re-exports
- `stores/fireProfile.ts`, `stores/accounts.ts` — Pinia stores
- `pages/Dashboard.vue`, `pages/Accounts.vue`, `pages/Settings.vue` — feature screens
- `components/` — shared UI (StatCard, NetWorthChart, AccountForm, BalanceForm)
- `router.ts`, `main.ts`, `App.vue` — shell + nav

---

## SLICE 1 — Foundation

### Task 1: Migration-strategy spike

**Files:** none committed (research task producing a decision note)

- [ ] **Step 1: Investigate crate compatibility**

Determine whether `refinery` or `sqlx` can run migrations against a `libsql` local connection. Key question: neither crate has a `libsql` runner/driver — `refinery` targets `rusqlite`/`postgres`/`mysql`, `sqlx` targets its own sqlite/postgres drivers. Check current crate docs and the `libsql` crate README for any migration helper.

Run:
```bash
cargo search refinery
cargo search libsql
```
And read https://docs.rs/libsql and https://docs.rs/refinery .

- [ ] **Step 2: Record the decision**

Write a 3-5 line note at the top of `src-tauri/src/migrations.rs` (created in Task 5) stating which approach was chosen and why. **Expected outcome:** if no crate cleanly drives a `libsql` connection, use the hand-rolled ordered-SQL runner specified in Task 5 (the default this plan implements). If a crate does work, adapt Task 5 to use it and keep the same `0001_init.sql`.

- [ ] **Step 3: No commit** (decision is captured in Task 5's code/comments)

---

### Task 2: Scaffold Tauri 2 + Vue + TypeScript

**Files:**
- Create: entire `src-tauri/` and `src/` trees, `vite.config.ts`, `package.json`, `tsconfig.json` (via scaffolder)

- [ ] **Step 1: Generate the app**

Run (in the repo root; the tool scaffolds into the current directory when empty, otherwise into a subfolder you then move up):
```bash
npm create tauri-app@latest trackmyfi-app -- --template vue-ts --manager npm
```
If it created a subfolder, move its contents into the repo root (keep existing `docs/` and `.claude/`). Verify `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src/main.ts`, and `vite.config.ts` exist.

- [ ] **Step 2: Install JS deps and verify dev build**

Run:
```bash
npm install
npm run tauri dev
```
Expected: a desktop window opens showing the default Tauri+Vue template. Close it.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri 2 + Vue + TypeScript app"
```

---

### Task 3: Add frontend libraries (NuxtUI, Router, Pinia, Luxon, unovis, Vitest)

**Files:**
- Modify: `package.json`, `vite.config.ts`, `src/main.ts`, `src/App.vue`
- Create: `src/router.ts`, `vitest.config.ts`

- [ ] **Step 1: Install dependencies**

Run:
```bash
npm install @nuxt/ui vue-router pinia luxon @unovis/vue @unovis/ts
npm install -D vitest @vue/test-utils jsdom @types/luxon
```

- [ ] **Step 2: Wire the Vite plugin and styles for NuxtUI**

Per current `@nuxt/ui` Vue (non-Nuxt) setup docs (https://ui.nuxt.com/getting-started/installation/vue), add its Vite plugin to `vite.config.ts` and import its CSS. Typical shape:
```ts
import ui from '@nuxt/ui/vite'
// inside plugins: [ vue(), ui() ]
```
And in `src/main.css` (create, imported from `main.ts`):
```css
@import "tailwindcss";
@import "@nuxt/ui";
```

- [ ] **Step 3: Create the router**

Create `src/router.ts`:
```ts
import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/settings', name: 'settings', component: () => import('./pages/Settings.vue') },
]

export const router = createRouter({ history: createWebHashHistory(), routes })
```
Use hash history — simplest for a `tauri://` webview with no server.

- [ ] **Step 4: Wire main.ts**

Replace `src/main.ts`:
```ts
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ui from '@nuxt/ui/vue-plugin'
import App from './App.vue'
import { router } from './router'
import './main.css'

createApp(App).use(createPinia()).use(router).use(ui).mount('#app')
```
(Verify the `@nuxt/ui/vue-plugin` import path against the installed version.)

- [ ] **Step 5: Create vitest config**

Create `vitest.config.ts`:
```ts
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
})
```
Add to `package.json` scripts: `"test": "vitest run"`, `"test:watch": "vitest"`.

- [ ] **Step 6: Verify**

Run `npm run test` — expected: "No test files found" (exit 0 is fine) — and `npm run tauri dev` still opens (App.vue may need the placeholder shell from Task 4 first; if it errors on missing pages, proceed to Task 4 then re-verify).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: add NuxtUI, router, Pinia, Luxon, unovis, Vitest"
```

---

### Task 4: App shell + navigation

**Files:**
- Modify: `src/App.vue`
- Create: `src/pages/Dashboard.vue`, `src/pages/Accounts.vue`, `src/pages/Settings.vue`

- [ ] **Step 1: Create placeholder pages**

Create each of `src/pages/Dashboard.vue`, `src/pages/Accounts.vue`, `src/pages/Settings.vue` with a heading, e.g. `src/pages/Dashboard.vue`:
```vue
<template>
  <div class="p-6">
    <h1 class="text-2xl font-bold">Dashboard</h1>
  </div>
</template>
```
(Repeat with "Accounts" and "Settings" headings.)

- [ ] **Step 2: Build the shell with nav**

Replace `src/App.vue` with a sidebar nav showing all 8 items; Phase 1 items link, the rest are disabled:
```vue
<script setup lang="ts">
const links = [
  { label: 'Dashboard', to: '/', icon: 'i-lucide-layout-dashboard' },
  { label: 'Accounts', to: '/accounts', icon: 'i-lucide-wallet' },
  { label: 'Transactions', icon: 'i-lucide-receipt', disabled: true },
  { label: 'Paychecks', icon: 'i-lucide-banknote', disabled: true },
  { label: 'Contributions', icon: 'i-lucide-piggy-bank', disabled: true },
  { label: 'Budget', icon: 'i-lucide-calculator', disabled: true },
  { label: 'Forecast', icon: 'i-lucide-trending-up', disabled: true },
  { label: 'Settings', to: '/settings', icon: 'i-lucide-settings' },
]
</script>

<template>
  <UApp>
    <div class="flex h-screen">
      <nav class="w-56 border-r border-default p-3 space-y-1">
        <template v-for="l in links" :key="l.label">
          <RouterLink v-if="l.to" :to="l.to" class="flex items-center gap-2 rounded px-3 py-2 hover:bg-elevated">
            <UIcon :name="l.icon" /> {{ l.label }}
          </RouterLink>
          <span v-else class="flex items-center gap-2 rounded px-3 py-2 text-muted opacity-50">
            <UIcon :name="l.icon" /> {{ l.label }}
          </span>
        </template>
      </nav>
      <main class="flex-1 overflow-auto"><RouterView /></main>
    </div>
  </UApp>
</template>
```
(Adjust NuxtUI utility class names to the installed version's design tokens if they differ.)

- [ ] **Step 3: Verify**

Run `npm run tauri dev`. Expected: window with sidebar; clicking Dashboard/Accounts/Settings swaps the main panel; other items appear disabled.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: app shell with sidebar navigation"
```

---

### Task 5: Database connection + migration runner

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
- Create: `src-tauri/src/db.rs`, `src-tauri/src/migrations.rs`, `src-tauri/migrations/0001_init.sql`

- [ ] **Step 1: Add Rust dependencies**

In `src-tauri/Cargo.toml` add (verify versions against crates.io):
```toml
libsql = { version = "0.6", features = ["serde", "core"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ts-rs = "10"
```
Tauri 2's async runtime is available via `tauri::async_runtime`.

- [ ] **Step 2: Write the init SQL**

Create `src-tauri/migrations/0001_init.sql`:
```sql
CREATE TABLE fire_profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  current_age INTEGER NOT NULL,
  target_retirement_age INTEGER NOT NULL,
  annual_expenses_target REAL NOT NULL,
  lean_fire_annual_expenses REAL,
  fat_fire_annual_expenses REAL,
  annual_income REAL NOT NULL,
  expected_return_rate REAL NOT NULL,
  inflation_rate REAL NOT NULL
);

CREATE TABLE account (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  institution TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  include_in_fire_calculations INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE account_balance (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  balance REAL NOT NULL,
  recorded_at TEXT NOT NULL
);

CREATE INDEX idx_balance_account ON account_balance(account_id, recorded_at);
```

- [ ] **Step 3: Write the migration runner**

Create `src-tauri/src/migrations.rs` (hand-rolled ordered runner — the spike default; replace with the crate if the spike found one):
```rust
// Migration strategy (decided in Task 1 spike): hand-rolled ordered-SQL runner.
// Reason: neither refinery nor sqlx drives a libsql connection directly.
use libsql::Connection;

struct Migration { version: i64, name: &'static str, sql: &'static str }

const MIGRATIONS: &[Migration] = &[
    Migration { version: 1, name: "init", sql: include_str!("../migrations/0001_init.sql") },
];

pub async fn run(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        (),
    ).await.map_err(|e| e.to_string())?;

    let mut applied = std::collections::HashSet::new();
    let mut rows = conn.query("SELECT version FROM schema_migrations", ())
        .await.map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        applied.insert(row.get::<i64>(0).map_err(|e| e.to_string())?);
    }

    for m in MIGRATIONS {
        if applied.contains(&m.version) { continue; }
        conn.execute_batch(m.sql).await.map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            libsql::params![m.version, m.name],
        ).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
```
(Verify `execute`, `query`, `params!`, and `execute_batch` signatures against the installed `libsql` version; adjust if the API differs.)

- [ ] **Step 4: Write the db module**

Create `src-tauri/src/db.rs`:
```rust
use libsql::{Builder, Connection, Database};
use tauri::{AppHandle, Manager};

pub struct Db(pub Database);

impl Db {
    pub async fn conn(&self) -> Result<Connection, String> {
        self.0.connect().map_err(|e| e.to_string())
    }
}

pub async fn init(app: &AppHandle) -> Result<Db, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("trackmyfi.db");
    let db = Builder::new_local(path).build().await.map_err(|e| e.to_string())?;
    let conn = db.connect().map_err(|e| e.to_string())?;
    crate::migrations::run(&conn).await?;
    Ok(Db(db))
}
```

- [ ] **Step 5: Initialize on startup**

In `src-tauri/src/lib.rs`, declare the modules and manage the Db in `setup`:
```rust
mod db;
mod migrations;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let db = tauri::async_runtime::block_on(db::init(&handle))?;
            app.manage(db);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
(Keep existing plugins/invoke_handler lines; only add the `mod` declarations, the `setup` body, and later the `.invoke_handler`.)

- [ ] **Step 6: Verify it builds and migrates**

Run `npm run tauri dev`. Expected: window opens with no panic. Confirm the DB file exists (path printed by adding a temporary `println!` of `path` in `init`, then remove it). Then check tables:
```bash
# locate app_data_dir for the app, then:
sqlite3 "<app_data_dir>/trackmyfi.db" ".tables"
```
Expected output includes: `account  account_balance  fire_profile  schema_migrations`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: libSQL connection and migration runner with initial schema"
```

---

### Task 6: Models + ts-rs type generation

**Files:**
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`
- Create (generated): `src/lib/types/*`

- [ ] **Step 1: Define serde + ts-rs models**

Create `src-tauri/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct FireProfile {
    pub current_age: i64,
    pub target_retirement_age: i64,
    pub annual_expenses_target: f64,
    pub lean_fire_annual_expenses: Option<f64>,
    pub fat_fire_annual_expenses: Option<f64>,
    pub annual_income: f64,
    pub expected_return_rate: f64,
    pub inflation_rate: f64,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub institution: Option<String>,
    pub is_active: bool,
    pub include_in_fire_calculations: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct AccountBalance {
    pub id: i64,
    pub account_id: i64,
    pub balance: f64,
    pub recorded_at: String,
}
```
Add `mod models;` to `src-tauri/src/lib.rs`.

- [ ] **Step 2: Generate the TS types**

Run:
```bash
cd src-tauri && cargo test export_bindings && cd ..
```
`ts-rs` exports bindings during `cargo test`. Expected: `src/lib/types/FireProfile.ts`, `Account.ts`, `AccountBalance.ts` created. (If your `ts-rs` version needs an explicit test, add `#[cfg(test)] mod ts { #[test] fn export() {} }` — generation runs via the derive's generated tests.)

- [ ] **Step 3: Verify generated files**

Confirm `src/lib/types/FireProfile.ts` exists and exports an interface with camelCase-or-snake fields matching the struct. Note the field casing ts-rs produced — the api wrappers in later tasks must match it exactly.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: Rust models with ts-rs type generation"
```

---

## SLICE 2 — FIRE Profile

### Task 7: FIRE Profile Rust commands + round-trip test

**Files:**
- Create: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/fire_profile.rs`, `src-tauri/tests/roundtrip.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/migrations/0001_init.sql` (seed default row)

- [ ] **Step 1: Seed a default profile row**

Append to `src-tauri/migrations/0001_init.sql`:
```sql
INSERT INTO fire_profile
  (id, current_age, target_retirement_age, annual_expenses_target,
   annual_income, expected_return_rate, inflation_rate)
VALUES (1, 30, 50, 40000, 80000, 0.07, 0.03);
```
(Delete any existing local `trackmyfi.db` so the migration re-runs cleanly during dev, or bump to a new migration if data exists.)

- [ ] **Step 2: Write the failing round-trip test**

Create `src-tauri/tests/roundtrip.rs`:
```rust
// Integration smoke test: temp DB, migrate, round-trip fire_profile.
use libsql::Builder;

#[tokio::test]
async fn fire_profile_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    trackmyfi_lib::migrations::run(&conn).await.unwrap();

    conn.execute(
        "UPDATE fire_profile SET annual_expenses_target = 50000 WHERE id = 1",
        (),
    ).await.unwrap();

    let mut rows = conn.query("SELECT annual_expenses_target FROM fire_profile WHERE id = 1", ())
        .await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let v: f64 = row.get(0).unwrap();
    assert_eq!(v, 50000.0);
}
```
Make `migrations` public: ensure `pub mod migrations;` in `lib.rs`, and that the lib target name is `trackmyfi_lib` (check `[lib] name` in `Cargo.toml`; adjust the `use` path to match). Add `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }` to `[dev-dependencies]`.

- [ ] **Step 3: Run the test, expect failure first**

Run:
```bash
cd src-tauri && cargo test fire_profile_roundtrip
```
Expected initially: compile error (module not public / wrong lib name). Fix visibility until it compiles and passes. Expected after fix: PASS.

- [ ] **Step 4: Write the commands**

Create `src-tauri/src/commands/fire_profile.rs`:
```rust
use crate::db::Db;
use crate::models::FireProfile;
use tauri::State;

#[tauri::command]
pub async fn get_fire_profile(db: State<'_, Db>) -> Result<FireProfile, String> {
    let conn = db.conn().await?;
    let mut rows = conn.query(
        "SELECT current_age, target_retirement_age, annual_expenses_target, \
         lean_fire_annual_expenses, fat_fire_annual_expenses, annual_income, \
         expected_return_rate, inflation_rate FROM fire_profile WHERE id = 1", (),
    ).await.map_err(|e| e.to_string())?;
    let row = rows.next().await.map_err(|e| e.to_string())?
        .ok_or("fire_profile row missing")?;
    libsql::de::from_row::<FireProfile>(&row).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_fire_profile(db: State<'_, Db>, profile: FireProfile) -> Result<(), String> {
    let conn = db.conn().await?;
    conn.execute(
        "UPDATE fire_profile SET current_age=?1, target_retirement_age=?2, \
         annual_expenses_target=?3, lean_fire_annual_expenses=?4, fat_fire_annual_expenses=?5, \
         annual_income=?6, expected_return_rate=?7, inflation_rate=?8 WHERE id = 1",
        libsql::params![
            profile.current_age, profile.target_retirement_age, profile.annual_expenses_target,
            profile.lean_fire_annual_expenses, profile.fat_fire_annual_expenses,
            profile.annual_income, profile.expected_return_rate, profile.inflation_rate
        ],
    ).await.map_err(|e| e.to_string())?;
    Ok(())
}
```
Create `src-tauri/src/commands/mod.rs`:
```rust
pub mod fire_profile;
pub mod accounts;
```
(`accounts` is filled in Task 9; create an empty `accounts.rs` with `// filled in Task 9` for now, or add the `pub mod accounts;` line in Task 9.)

- [ ] **Step 5: Register commands**

In `src-tauri/src/lib.rs` add `mod commands;` and the handler:
```rust
.invoke_handler(tauri::generate_handler![
    commands::fire_profile::get_fire_profile,
    commands::fire_profile::upsert_fire_profile,
])
```

- [ ] **Step 6: Verify build**

Run `cd src-tauri && cargo build`. Expected: compiles. (Verify `libsql::de::from_row` exists in the installed version; if not, map fields manually with `row.get(i)`.)

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: fire profile commands with round-trip test"
```

---

### Task 8: FIRE Profile frontend (api → store → Settings page)

**Files:**
- Create: `src/lib/api/fireProfile.ts`, `src/stores/fireProfile.ts`
- Modify: `src/pages/Settings.vue`

- [ ] **Step 1: Typed api wrapper**

Create `src/lib/api/fireProfile.ts` (match field casing to the generated type from Task 6):
```ts
import { invoke } from '@tauri-apps/api/core'
import type { FireProfile } from '../types/FireProfile'

export const getFireProfile = () => invoke<FireProfile>('get_fire_profile')
export const upsertFireProfile = (profile: FireProfile) =>
  invoke<void>('upsert_fire_profile', { profile })
```

- [ ] **Step 2: Pinia store**

Create `src/stores/fireProfile.ts`:
```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { FireProfile } from '../lib/types/FireProfile'
import { getFireProfile, upsertFireProfile } from '../lib/api/fireProfile'

export const useFireProfileStore = defineStore('fireProfile', () => {
  const profile = ref<FireProfile | null>(null)
  async function load() { profile.value = await getFireProfile() }
  async function save(next: FireProfile) {
    await upsertFireProfile(next)
    profile.value = next
  }
  return { profile, load, save }
})
```

- [ ] **Step 3: Settings page form**

Replace `src/pages/Settings.vue` with a form bound to a local copy of the profile, calling `save` on submit. Use NuxtUI `UForm`/`UInput`/`UButton`. Load on mount:
```vue
<script setup lang="ts">
import { reactive, onMounted } from 'vue'
import { useFireProfileStore } from '../stores/fireProfile'
import type { FireProfile } from '../lib/types/FireProfile'

const store = useFireProfileStore()
const form = reactive<FireProfile>({} as FireProfile)

onMounted(async () => {
  await store.load()
  Object.assign(form, store.profile)
})
async function onSubmit() { await store.save({ ...form }) }
</script>

<template>
  <div class="p-6 max-w-xl">
    <h1 class="text-2xl font-bold mb-4">FIRE Profile</h1>
    <UForm :state="form" @submit="onSubmit" class="space-y-3">
      <UFormField label="Current age"><UInput v-model.number="form.currentAge" type="number" /></UFormField>
      <UFormField label="Target retirement age"><UInput v-model.number="form.targetRetirementAge" type="number" /></UFormField>
      <UFormField label="Annual expenses target"><UInput v-model.number="form.annualExpensesTarget" type="number" /></UFormField>
      <UFormField label="Lean FIRE expenses (optional)"><UInput v-model.number="form.leanFireAnnualExpenses" type="number" /></UFormField>
      <UFormField label="Fat FIRE expenses (optional)"><UInput v-model.number="form.fatFireAnnualExpenses" type="number" /></UFormField>
      <UFormField label="Annual income"><UInput v-model.number="form.annualIncome" type="number" /></UFormField>
      <UFormField label="Expected return rate (e.g. 0.07)"><UInput v-model.number="form.expectedReturnRate" type="number" step="0.01" /></UFormField>
      <UFormField label="Inflation rate (e.g. 0.03)"><UInput v-model.number="form.inflationRate" type="number" step="0.01" /></UFormField>
      <UButton type="submit">Save</UButton>
    </UForm>
  </div>
</template>
```
(Match `form` field names to the generated type's casing exactly.)

- [ ] **Step 4: Verify end-to-end**

Run `npm run tauri dev`. Go to Settings, change a value, Save, reload the app (Cmd/Ctrl+R). Expected: the saved value persists.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: FIRE profile settings page (api, store, form)"
```

---

## SLICE 3 — Accounts + balance snapshots

### Task 9: Accounts Rust commands + round-trip test

**Files:**
- Create: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/tests/roundtrip.rs`

- [ ] **Step 1: Add a failing accounts round-trip test**

Append to `src-tauri/tests/roundtrip.rs`:
```rust
#[tokio::test]
async fn account_and_balance_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    trackmyfi_lib::migrations::run(&conn).await.unwrap();

    conn.execute(
        "INSERT INTO account (name, type, is_active, include_in_fire_calculations, created_at) \
         VALUES ('Brokerage', 'brokerage', 1, 1, '2026-01-01')", (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO account_balance (account_id, balance, recorded_at) \
         VALUES (1, 12345.67, '2026-01-01')", (),
    ).await.unwrap();

    let mut rows = conn.query("SELECT balance FROM account_balance WHERE account_id = 1", ())
        .await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<f64>(0).unwrap(), 12345.67);
}
```

- [ ] **Step 2: Run, expect pass** (schema already supports this)

Run `cd src-tauri && cargo test account_and_balance_roundtrip`. Expected: PASS. (This guards the schema/migration; the commands are tested via the app in Step 5.)

- [ ] **Step 3: Write the commands**

Create `src-tauri/src/commands/accounts.rs`:
```rust
use crate::db::Db;
use crate::models::{Account, AccountBalance};
use tauri::State;

#[tauri::command]
pub async fn list_accounts(db: State<'_, Db>) -> Result<Vec<Account>, String> {
    let conn = db.conn().await?;
    let mut rows = conn.query(
        "SELECT id, name, type, institution, is_active, include_in_fire_calculations, created_at \
         FROM account ORDER BY created_at", (),
    ).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(libsql::de::from_row::<Account>(&row).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn create_account(
    db: State<'_, Db>, name: String, r#type: String,
    institution: Option<String>, include_in_fire_calculations: bool, created_at: String,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO account (name, type, institution, is_active, include_in_fire_calculations, created_at) \
         VALUES (?1, ?2, ?3, 1, ?4, ?5)",
        libsql::params![name, r#type, institution, include_in_fire_calculations, created_at],
    ).await.map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub async fn archive_account(db: State<'_, Db>, id: i64) -> Result<(), String> {
    let conn = db.conn().await?;
    conn.execute("UPDATE account SET is_active = 0 WHERE id = ?1", libsql::params![id])
        .await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn add_account_balance(
    db: State<'_, Db>, account_id: i64, balance: f64, recorded_at: String,
) -> Result<(), String> {
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO account_balance (account_id, balance, recorded_at) VALUES (?1, ?2, ?3)",
        libsql::params![account_id, balance, recorded_at],
    ).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_account_balances(db: State<'_, Db>, account_id: i64) -> Result<Vec<AccountBalance>, String> {
    let conn = db.conn().await?;
    let mut rows = conn.query(
        "SELECT id, account_id, balance, recorded_at FROM account_balance \
         WHERE account_id = ?1 ORDER BY recorded_at",
        libsql::params![account_id],
    ).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(libsql::de::from_row::<AccountBalance>(&row).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_all_balances(db: State<'_, Db>) -> Result<Vec<AccountBalance>, String> {
    let conn = db.conn().await?;
    let mut rows = conn.query(
        "SELECT id, account_id, balance, recorded_at FROM account_balance ORDER BY recorded_at", (),
    ).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(libsql::de::from_row::<AccountBalance>(&row).map_err(|e| e.to_string())?);
    }
    Ok(out)
}
```
(`list_all_balances` feeds the dashboard net-worth-over-time chart in Slice 4.)

- [ ] **Step 4: Register commands**

Add to the `generate_handler!` list in `src-tauri/src/lib.rs`:
```rust
commands::accounts::list_accounts,
commands::accounts::create_account,
commands::accounts::archive_account,
commands::accounts::add_account_balance,
commands::accounts::list_account_balances,
commands::accounts::list_all_balances,
```

- [ ] **Step 5: Verify build + tests**

Run `cd src-tauri && cargo build && cargo test`. Expected: builds, all round-trip tests PASS.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: account and balance commands"
```

---

### Task 10: Accounts frontend (api → store → Accounts page)

**Files:**
- Create: `src/lib/api/accounts.ts`, `src/stores/accounts.ts`, `src/components/AccountForm.vue`, `src/components/BalanceForm.vue`, `src/lib/accountTypes.ts`
- Modify: `src/pages/Accounts.vue`

- [ ] **Step 1: Account-type metadata**

Create `src/lib/accountTypes.ts`:
```ts
export const ACCOUNT_TYPES = [
  'checking', 'savings', 'brokerage', '401k', 'roth_401k',
  'traditional_ira', 'roth_ira', 'hsa', 'real_estate', 'crypto', 'liability',
] as const
export type AccountType = typeof ACCOUNT_TYPES[number]

export const INVESTMENT_TYPES = new Set<string>(
  ['brokerage','401k','roth_401k','traditional_ira','roth_ira','hsa','crypto'],
)
export const isInvestment = (t: string) => INVESTMENT_TYPES.has(t)
export const isLiability = (t: string) => t === 'liability'
export const defaultIncludeInFire = (t: AccountType) => INVESTMENT_TYPES.has(t)
```

- [ ] **Step 2: Typed api wrapper**

Create `src/lib/api/accounts.ts`:
```ts
import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types/Account'
import type { AccountBalance } from '../types/AccountBalance'

export const listAccounts = () => invoke<Account[]>('list_accounts')
export const createAccount = (a: {
  name: string; type: string; institution: string | null;
  includeInFireCalculations: boolean; createdAt: string
}) => invoke<number>('create_account', a)
export const archiveAccount = (id: number) => invoke<void>('archive_account', { id })
export const addAccountBalance = (b: { accountId: number; balance: number; recordedAt: string }) =>
  invoke<void>('add_account_balance', b)
export const listAccountBalances = (accountId: number) =>
  invoke<AccountBalance[]>('list_account_balances', { accountId })
export const listAllBalances = () => invoke<AccountBalance[]>('list_all_balances')
```
(Tauri maps camelCase JS args to snake_case Rust params automatically; verify against the installed `@tauri-apps/api` version and adjust if needed.)

- [ ] **Step 3: Pinia store**

Create `src/stores/accounts.ts`:
```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import * as api from '../lib/api/accounts'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<Account[]>([])
  const allBalances = ref<AccountBalance[]>([])

  async function load() {
    accounts.value = await api.listAccounts()
    allBalances.value = await api.listAllBalances()
  }
  async function create(a: Parameters<typeof api.createAccount>[0]) {
    await api.createAccount(a); await load()
  }
  async function archive(id: number) { await api.archiveAccount(id); await load() }
  async function addBalance(b: Parameters<typeof api.addAccountBalance>[0]) {
    await api.addAccountBalance(b); await load()
  }
  return { accounts, allBalances, load, create, archive, addBalance }
})
```

- [ ] **Step 4: Accounts page + forms**

Replace `src/pages/Accounts.vue` to: list active accounts with their latest balance, an "Add account" form (`AccountForm.vue`: name, type select that pre-fills `includeInFireCalculations` via `defaultIncludeInFire`, institution, include toggle), and a per-account "Add balance" form (`BalanceForm.vue`: balance + date, defaulting date to today via Luxon `DateTime.now().toISODate()`). Wire all actions through the store. Compute latest balance per account from `allBalances` (max `recordedAt`).

- [ ] **Step 5: Verify end-to-end**

Run `npm run tauri dev`. Create an account, add two balance snapshots on different dates, archive a different account. Reload. Expected: accounts and latest balances persist; archived account disappears from the active list.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: accounts page with balance snapshots"
```

---

## SLICE 4 — FIRE Dashboard

### Task 11: FIRE math types + simple metrics (TDD)

**Files:**
- Create: `src/lib/fire/types.ts`, `src/lib/fire/metrics.ts`, `src/lib/fire/metrics.test.ts`

- [ ] **Step 1: Define pure input types**

Create `src/lib/fire/types.ts`:
```ts
export interface FireAccount { id: number; type: string; includeInFireCalculations: boolean }
export interface FireBalance { accountId: number; balance: number; recordedAt: string } // ISO date
```

- [ ] **Step 2: Write failing tests**

Create `src/lib/fire/metrics.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { fireNumber, latestBalances, currentNetWorth, investableNetWorth, fiProgress } from './metrics'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'checking', includeInFireCalculations: false },
  { id: 3, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 1, balance: 200, recordedAt: '2026-02-01' }, // latest for acct 1
  { accountId: 2, balance: 50, recordedAt: '2026-01-15' },
  { accountId: 3, balance: 30, recordedAt: '2026-01-10' },  // liability
]

describe('fire metrics', () => {
  it('fireNumber is 25x expenses', () => {
    expect(fireNumber(40000)).toBe(1_000_000)
  })
  it('latestBalances picks most recent per account', () => {
    const m = latestBalances(balances)
    expect(m.get(1)).toBe(200)
    expect(m.get(2)).toBe(50)
  })
  it('currentNetWorth subtracts liabilities', () => {
    // 200 (acct1) + 50 (acct2) - 30 (liability acct3) = 220
    expect(currentNetWorth(accounts, balances)).toBe(220)
  })
  it('investableNetWorth counts only included accounts', () => {
    expect(investableNetWorth(accounts, balances)).toBe(200)
  })
  it('fiProgress is investable / fireNumber * 100', () => {
    expect(fiProgress(200_000, 1_000_000)).toBe(20)
  })
})
```

- [ ] **Step 3: Run, expect fail**

Run `npm run test`. Expected: FAIL — `./metrics` not found.

- [ ] **Step 4: Implement**

Create `src/lib/fire/metrics.ts`:
```ts
import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'

export const fireNumber = (annualExpensesTarget: number) => annualExpensesTarget * 25

export function latestBalances(balances: FireBalance[]): Map<number, number> {
  const latestAt = new Map<number, string>()
  const value = new Map<number, number>()
  for (const b of balances) {
    const seen = latestAt.get(b.accountId)
    if (!seen || b.recordedAt > seen) {
      latestAt.set(b.accountId, b.recordedAt)
      value.set(b.accountId, b.balance)
    }
  }
  return value
}

export function currentNetWorth(accounts: FireAccount[], balances: FireBalance[]): number {
  const latest = latestBalances(balances)
  let total = 0
  for (const a of accounts) {
    const bal = latest.get(a.id) ?? 0
    total += isLiability(a.type) ? -bal : bal
  }
  return total
}

export function investableNetWorth(accounts: FireAccount[], balances: FireBalance[]): number {
  const latest = latestBalances(balances)
  let total = 0
  for (const a of accounts) {
    if (!a.includeInFireCalculations) continue
    const bal = latest.get(a.id) ?? 0
    total += isLiability(a.type) ? -bal : bal
  }
  return total
}

export const fiProgress = (investable: number, fireNum: number) =>
  fireNum === 0 ? 0 : (investable / fireNum) * 100
```

- [ ] **Step 5: Run, expect pass**

Run `npm run test`. Expected: all 5 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: FIRE metrics (net worth, investable, FI progress) with tests"
```

---

### Task 12: Net worth over time (TDD)

**Files:**
- Create: `src/lib/fire/netWorthSeries.ts`, `src/lib/fire/netWorthSeries.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/fire/netWorthSeries.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { netWorthOverTime } from './netWorthSeries'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 2, balance: 40, recordedAt: '2026-01-15' },
  { accountId: 1, balance: 300, recordedAt: '2026-02-01' },
]

describe('netWorthOverTime', () => {
  it('computes net worth at each distinct date using carry-forward', () => {
    const series = netWorthOverTime(accounts, balances)
    expect(series).toEqual([
      { date: '2026-01-01', netWorth: 100 },               // acct1=100
      { date: '2026-01-15', netWorth: 60 },                // 100 - 40
      { date: '2026-02-01', netWorth: 260 },               // 300 - 40
    ])
  })
  it('returns empty for no balances', () => {
    expect(netWorthOverTime(accounts, [])).toEqual([])
  })
})
```

- [ ] **Step 2: Run, expect fail**

Run `npm run test`. Expected: FAIL — `./netWorthSeries` not found.

- [ ] **Step 3: Implement**

Create `src/lib/fire/netWorthSeries.ts`:
```ts
import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'

export interface NetWorthPoint { date: string; netWorth: number }

export function netWorthOverTime(accounts: FireAccount[], balances: FireBalance[]): NetWorthPoint[] {
  if (balances.length === 0) return []
  const typeById = new Map(accounts.map(a => [a.id, a.type]))
  const dates = [...new Set(balances.map(b => b.recordedAt))].sort()
  const sorted = [...balances].sort((a, b) => a.recordedAt.localeCompare(b.recordedAt))

  return dates.map(date => {
    const latestForAccount = new Map<number, number>()
    for (const b of sorted) {
      if (b.recordedAt <= date) latestForAccount.set(b.accountId, b.balance)
    }
    let netWorth = 0
    for (const [accountId, bal] of latestForAccount) {
      netWorth += isLiability(typeById.get(accountId) ?? '') ? -bal : bal
    }
    return { date, netWorth }
  })
}
```

- [ ] **Step 4: Run, expect pass**

Run `npm run test`. Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: net worth over time series with tests"
```

---

### Task 13: Projected FI date + savings rate (TDD)

**Files:**
- Create: `src/lib/fire/projection.ts`, `src/lib/fire/projection.test.ts`

- [ ] **Step 1: Write failing tests**

Create `src/lib/fire/projection.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { realMonthlyReturn, monthsToFire, savingsRate } from './projection'
import type { FireAccount, FireBalance } from './types'

describe('projection', () => {
  it('realMonthlyReturn deflates nominal by inflation', () => {
    // (1.07 / 1.03)^(1/12) - 1 ≈ 0.003180
    expect(realMonthlyReturn(0.07, 0.03)).toBeCloseTo(0.003180, 5)
  })

  it('monthsToFire returns 0 when already at the number', () => {
    expect(monthsToFire(1_000_000, 1000, 0.003, 1_000_000)).toBe(0)
  })

  it('monthsToFire grows the portfolio until it reaches the target', () => {
    // pure contributions, no return: need 100 months of 100 to go 0 -> 10000
    expect(monthsToFire(0, 100, 0, 10_000)).toBe(100)
  })

  it('monthsToFire returns null when unreachable within cap', () => {
    expect(monthsToFire(0, 0, 0, 10_000)).toBeNull()
  })

  it('savingsRate divides trailing-12mo investment increase by income', () => {
    const accounts: FireAccount[] = [
      { id: 1, type: 'brokerage', includeInFireCalculations: true },
      { id: 2, type: 'checking', includeInFireCalculations: false },
    ]
    const balances: FireBalance[] = [
      { accountId: 1, balance: 10_000, recordedAt: '2025-06-01' }, // ~12mo ago
      { accountId: 1, balance: 30_000, recordedAt: '2026-06-01' }, // now
      { accountId: 2, balance: 5_000, recordedAt: '2026-06-01' },  // non-investment, ignored
    ]
    // increase 20,000 / income 100,000 = 0.2
    expect(savingsRate(accounts, balances, 100_000, '2026-06-01')).toBeCloseTo(0.2, 6)
  })
})
```

- [ ] **Step 2: Run, expect fail**

Run `npm run test`. Expected: FAIL — `./projection` not found.

- [ ] **Step 3: Implement**

Create `src/lib/fire/projection.ts`:
```ts
import { DateTime } from 'luxon'
import type { FireAccount, FireBalance } from './types'
import { isInvestment } from '../accountTypes'

const MAX_MONTHS = 1200 // 100-year cap

export function realMonthlyReturn(expectedReturnRate: number, inflationRate: number): number {
  return Math.pow((1 + expectedReturnRate) / (1 + inflationRate), 1 / 12) - 1
}

/** Months until FV >= target, compounding monthly with a fixed contribution. null if unreachable within cap. */
export function monthsToFire(
  presentValue: number, monthlyContribution: number, monthlyReturn: number, target: number,
): number | null {
  if (presentValue >= target) return 0
  let fv = presentValue
  for (let m = 1; m <= MAX_MONTHS; m++) {
    fv = fv * (1 + monthlyReturn) + monthlyContribution
    if (fv >= target) return m
  }
  return null
}

export function projectedFiDate(
  presentValue: number, monthlyContribution: number,
  expectedReturnRate: number, inflationRate: number, target: number,
  from: DateTime = DateTime.now(),
): DateTime | null {
  const months = monthsToFire(presentValue, monthlyContribution, realMonthlyReturn(expectedReturnRate, inflationRate), target)
  return months === null ? null : from.plus({ months })
}

/** Sum of investment-account balances as of a date (most recent snapshot <= date). */
function investmentBalanceAt(accounts: FireAccount[], balances: FireBalance[], isoDate: string): number {
  const invest = new Set(accounts.filter(a => isInvestment(a.type)).map(a => a.id))
  const latest = new Map<number, { at: string; bal: number }>()
  for (const b of balances) {
    if (!invest.has(b.accountId) || b.recordedAt > isoDate) continue
    const seen = latest.get(b.accountId)
    if (!seen || b.recordedAt > seen.at) latest.set(b.accountId, { at: b.recordedAt, bal: b.balance })
  }
  let total = 0
  for (const { bal } of latest.values()) total += bal
  return total
}

/** Phase 1 approximation: trailing-12-month increase in investment balances / annual income. */
export function savingsRate(
  accounts: FireAccount[], balances: FireBalance[], annualIncome: number, asOfIso: string,
): number {
  if (annualIncome === 0) return 0
  const now = investmentBalanceAt(accounts, balances, asOfIso)
  const yearAgoIso = DateTime.fromISO(asOfIso).minus({ years: 1 }).toISODate()!
  const prior = investmentBalanceAt(accounts, balances, yearAgoIso)
  return (now - prior) / annualIncome
}
```

- [ ] **Step 4: Run, expect pass**

Run `npm run test`. Expected: all projection tests PASS.

- [ ] **Step 5: Add the fire barrel export**

Create `src/lib/fire/index.ts`:
```ts
export * from './types'
export * from './metrics'
export * from './netWorthSeries'
export * from './projection'
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: projected FI date and savings-rate approximation with tests"
```

---

### Task 14: Dashboard page + net worth chart

**Files:**
- Create: `src/components/StatCard.vue`, `src/components/NetWorthChart.vue`
- Modify: `src/pages/Dashboard.vue`

- [ ] **Step 1: StatCard component**

Create `src/components/StatCard.vue`:
```vue
<script setup lang="ts">
defineProps<{ label: string; value: string; hint?: string }>()
</script>

<template>
  <UCard>
    <div class="text-sm text-muted">{{ label }}</div>
    <div class="text-2xl font-semibold mt-1">{{ value }}</div>
    <div v-if="hint" class="text-xs text-muted mt-1">{{ hint }}</div>
  </UCard>
</template>
```

- [ ] **Step 2: NetWorthChart component**

Create `src/components/NetWorthChart.vue` wrapping unovis. Props: `points: NetWorthPoint[]`. Map to a `VisXYContainer` + `VisLine` + `VisAxis` time series (x = parsed date, y = netWorth). Per current `@unovis/vue` docs (https://unovis.dev), the typical shape:
```vue
<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis } from '@unovis/vue'
import type { NetWorthPoint } from '../lib/fire/netWorthSeries'
import { DateTime } from 'luxon'

const props = defineProps<{ points: NetWorthPoint[] }>()
type D = { t: number; v: number }
const data = () => props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.netWorth }))
const x = (d: D) => d.t
const y = (d: D) => d.v
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="y" />
    <VisAxis type="x" :tickFormat="(t:number) => DateTime.fromMillis(t).toFormat('LLL yyyy')" />
    <VisAxis type="y" />
  </VisXYContainer>
</template>
```
(Verify component/prop names against the installed `@unovis/vue` version.)

- [ ] **Step 3: Dashboard page**

Replace `src/pages/Dashboard.vue` to load both stores on mount, derive metrics via `lib/fire`, and render stat cards + the chart:
```vue
<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import {
  fireNumber, currentNetWorth, investableNetWorth, fiProgress,
  netWorthOverTime, projectedFiDate, savingsRate,
} from '../lib/fire'
import type { FireAccount, FireBalance } from '../lib/fire/types'
import StatCard from '../components/StatCard.vue'
import NetWorthChart from '../components/NetWorthChart.vue'

const fp = useFireProfileStore()
const acc = useAccountsStore()
onMounted(async () => { await Promise.all([fp.load(), acc.load()]) })

const fireAccounts = computed<FireAccount[]>(() =>
  acc.accounts.map(a => ({ id: a.id, type: a.type, includeInFireCalculations: a.includeInFireCalculations })))
const fireBalances = computed<FireBalance[]>(() =>
  acc.allBalances.map(b => ({ accountId: b.accountId, balance: b.balance, recordedAt: b.recordedAt })))

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const fireNum = computed(() => fp.profile ? fireNumber(fp.profile.annualExpensesTarget) : 0)
const netWorth = computed(() => currentNetWorth(fireAccounts.value, fireBalances.value))
const investable = computed(() => investableNetWorth(fireAccounts.value, fireBalances.value))
const progress = computed(() => fiProgress(investable.value, fireNum.value))
const series = computed(() => netWorthOverTime(fireAccounts.value, fireBalances.value))
const rate = computed(() => fp.profile
  ? savingsRate(fireAccounts.value, fireBalances.value, fp.profile.annualIncome, DateTime.now().toISODate()!)
  : 0)
const fiDate = computed(() => {
  if (!fp.profile) return null
  const monthly = (fp.profile.annualIncome * rate.value) / 12
  return projectedFiDate(investable.value, monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <h1 class="text-2xl font-bold">Dashboard</h1>
    <div class="grid grid-cols-2 lg:grid-cols-3 gap-4">
      <StatCard label="FIRE Number" :value="fmt(fireNum)" />
      <StatCard label="Current Net Worth" :value="fmt(netWorth)" />
      <StatCard label="Investable Net Worth" :value="fmt(investable)" />
      <StatCard label="FI Progress" :value="`${progress.toFixed(1)}%`" />
      <StatCard label="Projected FI Date" :value="fiDate ? fiDate.toFormat('LLL yyyy') : '—'" />
      <StatCard label="Savings Rate" :value="`${(rate * 100).toFixed(1)}%`" hint="Approximate — refined in Phase 2" />
    </div>
    <div class="border border-default rounded-lg p-4">
      <h2 class="font-semibold mb-2">Net Worth Over Time</h2>
      <NetWorthChart :points="series" />
    </div>
  </div>
</template>
```
(Match `a.type` / `a.includeInFireCalculations` / `b.accountId` etc. to the generated type casing from Task 6.)

- [ ] **Step 4: Verify end-to-end**

Run `npm run tauri dev`. With a profile and a few accounts/balances entered (from Slices 2–3), the Dashboard shows all six metrics and a net worth line. Add another balance snapshot in Accounts, return to Dashboard — the chart and metrics update after reload.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: FIRE dashboard with metrics and net worth chart"
```

---

## Final verification

- [ ] **Step 1: Full test suite**

Run:
```bash
npm run test
cd src-tauri && cargo test && cd ..
```
Expected: all Vitest and cargo tests PASS.

- [ ] **Step 2: Manual smoke of the core loop**

Run `npm run tauri dev`. Fresh flow: Settings → set profile; Accounts → add a brokerage account + balance, a checking account + balance, a liability + balance; Dashboard → confirm FIRE number = expenses×25, net worth = assets−liability, investable excludes checking/liability, chart renders, savings rate shows the "approximate" hint.

- [ ] **Step 3: Confirm deferred scope is absent and intentional**

Verify no encryption/Turso/sync code shipped (deferred to post-Phase-1) and the four non-Phase-1 nav items are disabled.

---

## Notes for the executor

- **Field casing:** `ts-rs` decides the generated field names (likely camelCase via serde rename or snake_case as-is). Inspect the generated files in Task 6 and make all api wrappers, stores, and pages match exactly. This is the single most likely source of bugs.
- **Version-sensitive APIs** flagged inline (libsql query/params/`de::from_row`, `@nuxt/ui` Vue plugin import, `@unovis/vue` components, Tauri arg casing): verify each against the installed version when you reach it, rather than trusting the snippet verbatim.
- **DB resets during dev:** when you change `0001_init.sql`, delete the local `trackmyfi.db` in the app-data dir so the migration re-runs, or add a new numbered migration.
```
