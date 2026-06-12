# TrackMyFI — Phase 1 Design (Core FIRE Loop)

**Date:** 2026-06-12
**Parent design:** [`2026-06-09-trackmyfi-design.md`](./2026-06-09-trackmyfi-design.md)
**Stack:** Tauri 2.x (Rust core + system webview) · Vue 3 · NuxtUI · Vite · Pinia · Vue Router · libSQL (local file) · Luxon · unovis · Vitest

---

## Goal

Stand up the TrackMyFI desktop app from a clean repo and ship the **Core FIRE Loop** end-to-end: a user can configure their FIRE profile, add accounts and record balance snapshots, and see real FIRE metrics (FIRE number, FI progress %, projected FI date, net worth chart) on a dashboard.

The repo is currently empty except for design docs — the original AdonisJS scaffold was cleared. Phase 1 builds the foundation **and** the first three features.

---

## Scope decisions (locked during brainstorming)

1. **Local-only first.** Phase 1 ships a working app on a plain local libSQL file. **Encryption-at-rest, OS-keychain storage, and background Turso cloud sync are explicitly deferred** to a post-Phase-1 slice. This is the fastest path to a usable app and avoids front-loading the hardest, most independent infrastructure.
2. **Typed per-entity Rust commands.** The Vue ↔ Rust `invoke()` boundary uses typed `#[tauri::command]` functions per entity with serde structs (not a generic SQL bridge). Strong type safety across the boundary; Rust grows per feature.
3. **Schema via a Rust migration crate** (refinery or sqlx), **pending a compatibility spike** with the `libsql` embedded client. These crates target rusqlite/postgres/etc., not `libsql` directly, so the first task in the foundation slice verifies compatibility. **Fallback:** a hand-rolled ordered-SQL migration runner (numbered `.sql` files embedded in the binary, applied in order, tracked in a `schema_migrations` table).
4. **Charting:** unovis (`@unovis/vue`) — Vue-native, time-series friendly, carries into Phase 3 forecast charts.
5. **Type-sharing:** `ts-rs` generates TS interfaces from Rust structs so the typed boundary stays in sync automatically.

---

## Sequencing — four vertical slices

Each slice leaves the app in a working, runnable state.

### Slice 1 — Foundation
- **First task:** migration-crate compatibility spike against the `libsql` embedded client. Decide refinery/sqlx vs. the hand-rolled fallback before building further.
- Tauri 2 shell + Vue 3 / Vite / NuxtUI / Vue Router / Pinia scaffold.
- Rust `db` module: open the local libSQL file in the app-data dir, hold the connection in Tauri `State`, run migrations on startup.
- First migration: the three Phase 1 tables (below).
- `ts-rs` type-generation wired into the build.
- App boots, connects to the DB, runs migrations, renders an empty shell with the nav.

### Slice 2 — FIRE Profile
End-to-end vertical: Rust commands (`get_fire_profile`, `upsert_fire_profile`) → TS api wrapper → Pinia `fireProfile` store → Settings page form. Seeded default row on first run.

### Slice 3 — Accounts + balance snapshots
Rust commands (`list_accounts`, `create_account`, `update_account`, `archive_account`, `add_account_balance`, `list_account_balances`) → TS api → Pinia `accounts` store → Accounts page (list with latest balance, create/edit/archive, add snapshot, view balance history).

### Slice 4 — FIRE Dashboard
Consumes Profile + Accounts. Pure `lib/fire/` math + unovis net worth chart. No new tables.

**Nav in Phase 1:** Dashboard, Accounts, Settings active. Transactions, Paychecks, Contributions, Budget, Forecast are stubbed/hidden until their phases.

---

## Architecture

### Rust core (`src-tauri`)
- **`db` module** — opens the local libSQL file (app-data dir), holds the connection in Tauri `State`, runs migrations on startup.
- **`commands` module** — typed `#[tauri::command]` functions per entity, each with serde structs mirroring rows.
- **Type-sharing** — `ts-rs` derives generate TS interfaces from the Rust structs.

### Vue frontend
- **`lib/api/`** — thin typed `invoke()` wrappers per entity, using the generated types.
- **`stores/`** — Pinia stores (`fireProfile`, `accounts`).
- **`lib/fire/`** — pure FIRE math functions (the testable core; no I/O).
- **`pages/` + `components/`** — Settings (FIRE Profile), Accounts, Dashboard.

### Data flow
`Vue page → Pinia store → lib/api wrapper → invoke() → Rust command → libSQL → back`. FIRE math runs in `lib/fire/` over data already loaded into stores; it never touches the DB directly.

---

## Data model (Phase 1 subset)

Only three tables are created in Phase 1. Phase 2/3 tables (`transaction`, `paycheck`, `budget_month`) arrive in later migrations.

### `fire_profile` (single row, id = 1)
| Field | Notes |
|---|---|
| `currentAge` | int |
| `targetRetirementAge` | int |
| `annualExpensesTarget` | used for FIRE number (25×) |
| `leanFireAnnualExpenses` | nullable |
| `fatFireAnnualExpenses` | nullable |
| `annualIncome` | used for savings-rate approximation |
| `expectedReturnRate` | e.g. 0.07 |
| `inflationRate` | e.g. 0.03 |

`fireNumber` is **derived, never stored**. The row is **seeded with sensible defaults on first run** so the dashboard never renders NaN.

### `account`
| Field | Notes |
|---|---|
| `id` | |
| `name` | |
| `type` | `checking`·`savings`·`brokerage`·`401k`·`roth_401k`·`traditional_ira`·`roth_ira`·`hsa`·`real_estate`·`crypto`·`liability` |
| `institution` | nullable |
| `isActive` | archive flag |
| `includeInFireCalculations` | defaults from type (investments→true; real_estate/checking/savings/liability→false); individually overridable |
| `createdAt` | |

### `account_balance`
| Field | Notes |
|---|---|
| `id` | |
| `accountId` | FK → account |
| `balance` | |
| `recordedAt` | snapshot timestamp |

---

## FIRE math (`lib/fire/`, pure functions)

- **FIRE Number** = `annualExpensesTarget × 25`.
- **Current Net Worth** = Σ latest snapshot per account (asset balances − liability balances).
- **Investable Net Worth** = same, restricted to accounts with `includeInFireCalculations = true`.
- **FI Progress %** = `investableNetWorth / fireNumber × 100`.
- **Net Worth over time (chart series)** — for each distinct `recordedAt` across all accounts, net worth = Σ over accounts of (that account's most-recent snapshot with `recordedAt ≤ T`). This handles accounts having independent snapshot timelines. Liability balances subtract.
- **Projected FI Date** — monthly compound-growth model computed in **real (today's-dollar) terms**: real monthly return = `((1 + expectedReturnRate) / (1 + inflationRate))^(1/12) − 1`. Starting from current investable net worth as present value and a monthly contribution, iterate months until future value ≥ FIRE number; convert month count to a date. Real terms keeps the projection consistent with `fireNumber`, which is in today's dollars.
- **Savings Rate (Phase 1 approximation)** = (trailing-12-month increase in investment-account balances) ÷ `annualIncome`.
  - **Known limitation:** this conflates contributions with market gains and can overstate the true savings rate. Accepted for Phase 1; becomes precise in Phase 2 once contribution transactions exist. The UI labels this figure as approximate.
  - The monthly contribution feeding the projection is derived from this savings rate (`annualIncome × savingsRate / 12`).

---

## Testing

- **TDD on `lib/fire/`** (Vitest) — pure functions, no I/O: fireNumber, currentNetWorth, investableNetWorth, fiProgress, netWorthOverTime, projectedFiDate, savingsRate. Tests written before implementation per the TDD workflow.
- **Rust data layer** — an integration smoke test: open a temp libSQL file, run migrations, round-trip each entity (insert → read back) to confirm commands and schema agree.
- **E2E** is out of scope for Phase 1.

---

## Out of scope for Phase 1 (deferred)

- Encryption-at-rest, OS-keychain storage of secrets.
- Background Turso cloud sync / multi-machine reconciliation.
- Transactions, Paychecks, Contributions, Anti-Budget (Phase 2).
- Forecasting: Coast/Lean/Fat FIRE, what-if planner (Phase 3).
- CSV import, live bank/brokerage sync.
- App-level passcode.

---

## Risks & de-risking

- **Migration crate vs. `libsql`** — addressed by the up-front compatibility spike in Slice 1, with a hand-rolled ordered-SQL runner as a proven fallback.
- **Savings-rate accuracy** — known and accepted as approximate for Phase 1; labeled in the UI; resolved in Phase 2.
- **Rust↔TS type drift** — mitigated by `ts-rs` generation rather than hand-maintained parallel types.
