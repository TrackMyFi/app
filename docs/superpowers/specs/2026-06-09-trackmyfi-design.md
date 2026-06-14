# TrackMyFI — High-Level Design

**Date:** 2026-06-09 · **Revised:** 2026-06-12 (local-first Tauri pivot)
**Stack:** Tauri (Rust core + webview) · Vue 3 · NuxtUI · Vite · libSQL/Turso (embedded replica) · Luxon

---

## Overview

TrackMyFI is a personal FIRE (Financial Independence Retire Early) tracking app inspired by YNAB but simplified and tailored toward FIRE goals. It replaces zero-based budgeting with an anti-budget philosophy, and layers FIRE-specific metrics and forecasting on top of net worth and transaction tracking.

**Scope:** Single-user, local-first **desktop app** (macOS + Windows executables). No hosted server, no monthly cost. Architecture should remain clean enough to support multi-user expansion later without a full rewrite, but multi-tenancy is explicitly out of scope for now.

---

## Architecture

TrackMyFI is a **local-first Tauri desktop app**, not a hosted web app — distributed as native macOS and Windows executables with no server and no monthly hosting cost.

- **Shell:** Tauri (Rust core + system webview).
- **Frontend:** Vue 3 + NuxtUI + Vite, running in the webview. All UI, FIRE calculations, CSV parsing, and forecasting logic live here in TypeScript.
- **Data layer:** libSQL (SQLite-compatible) as an **embedded replica**. Reads are always local — fast and fully offline. Writes are local and sync to a Turso cloud replica in the background. Because the embedded-replica client is native, the DB layer lives in a thin Rust command layer in the Tauri core (the `libsql` crate + a handful of `#[tauri::command]` functions for queries and `sync()`), exposed to Vue via `invoke()`. The frontend never talks to the network directly for data.
- **Sync:** A free-tier **Turso** cloud replica reconciles edits across machines automatically. The app works fully offline; sync resumes when connectivity returns. This handles the "opened on two machines" case gracefully rather than last-write-wins.
- **Encryption at rest:** provided by **OS full-disk encryption** (FileVault on macOS, BitLocker on Windows), which encrypts the whole disk including `trackmyfi.db`. App-level libSQL file encryption was evaluated and **deliberately not adopted** — it requires CMake (or migrating off libSQL to SQLCipher) and adds little over FileVault when the key sits transparently in the keychain. See decision record `docs/superpowers/specs/2026-06-13-encryption-at-rest-design.md`. (A future Turso sync token, when that feature lands, would be stored in the OS keychain.)
- **No auth:** single-user local app — there is no login or `User` entity. (An optional app-level passcode is a possible later addition.)

This replaces the original AdonisJS / Inertia / Lucid / Tuyau server stack that the repo was scaffolded with. The Vue + NuxtUI frontend carries over; the Node server, ORM, Inertia bridge, and auth layers are removed.

---

## Budgeting Philosophy

TrackMyFI uses the **anti-budget (pay yourself first)** model:

1. Income arrives
2. Savings and investment contributions come off the top (predetermined targets)
3. Fixed recurring obligations are covered (rent, subscriptions, utilities)
4. Everything remaining is guilt-free spending money

The budget module is **informational, not prescriptive** — it shows the anti-budget math each month. There are no "over budget" alerts, no category limits to stress over. The goal is awareness of savings rate, not micromanagement of spending.

---

## Modules & Build Phases

### Phase 1 — Core FIRE Loop

The minimum set that produces real FIRE metrics with real data.

#### FIRE Profile
User's FIRE input parameters. All forecasting and dashboard metrics derive from these.

- `currentAge`
- `targetRetirementAge`
- `annualExpensesTarget` — used to calculate FIRE number (25× rule)
- `leanFireAnnualExpenses` — optional lower target for Lean FIRE projections
- `fatFireAnnualExpenses` — optional higher target for Fat FIRE projections
- `annualIncome`
- `expectedReturnRate` — assumed annual investment return (e.g. 7%)
- `inflationRate` — for real-return adjusted projections
- `fireNumber` — derived: `annualExpensesTarget × 25`

Lean FIRE and Fat FIRE projections use `leanFireAnnualExpenses` and `fatFireAnnualExpenses` respectively, falling back to `annualExpensesTarget` if not set.

#### Accounts
All financial accounts tracked in one place. Balances are recorded as **periodic snapshots** rather than derived from transactions — simpler and more accurate for investment accounts where value fluctuates independently of deposits.

Account types:
- `checking` · `savings` (liquid cash)
- `brokerage` (taxable investments)
- `401k` · `roth_401k` · `traditional_ira` · `roth_ira`
- `hsa`
- `real_estate`
- `crypto`
- `liability` (credit cards, loans, mortgage)

Each account has a name, type, institution, and active status. Balance history is stored as `AccountBalance` snapshots (`balance`, `recordedAt`).

Net worth = sum of all asset balances − sum of all liability balances.

#### FIRE Dashboard
The primary view. All metrics are derived from Accounts + FireProfile.

- **FIRE Number** — `annualExpensesTarget × 25`
- **Current Net Worth** — live from latest account snapshots
- **Investable Net Worth** — sum of balances for accounts flagged `includeInFireCalculations: true`. Investment account types (`brokerage`, `401k`, `roth_401k`, `traditional_ira`, `roth_ira`, `hsa`, `crypto`) default to included; `real_estate`, `checking`, `savings`, and `liability` default to excluded. Each account can be toggled individually.
- **FI Progress %** — `investableNetWorth / fireNumber × 100`
- **Projected FI Date** — compound growth projection to reach FIRE number at current savings rate
- **Savings Rate** — in Phase 1: approximated as (sum of investment balance increases over trailing 12 months ÷ `annualIncome` from FireProfile). Becomes more precise in Phase 2 once income and contribution transactions exist.
- **Net Worth Chart** — historical balance snapshots over time

---

### Phase 2 — Transactions, Paychecks & Budget

#### Transactions
The financial ledger. Supports manual entry and CSV import. The import layer is designed to be swappable — a future live sync provider (Teller.io, SimpleFIN Bridge) feeds the same transaction store with no changes to the rest of the app.

Fields: `amount`, `description`, `date`, `type` (income / expense / transfer), `category` (savings / fixed / discretionary), `isContribution`, `accountId`, `importSource` (manual / csv).

Contribution transactions (investment deposits) are regular transactions with `isContribution: true` and an associated `accountId`. This powers the Contributions view without duplicate data entry.

#### Paychecks
Structured paycheck records — distinct from the transaction ledger so paycheck breakdown data doesn't add noise to expense tracking. Supports manual entry and CSV import (ADP, Gusto, Paychex all export pay history as CSV).

Fields: `payDate`, `employer`, `payPeriod`, `grossAmount`, `netAmount`, `federalTax`, `stateTax`, `ficaTax`, `importSource` (manual / csv).

Deductions are stored as a structured array:
```
deductions: [{ label, amount, preTax, contributionAccountType? }]
```

**Auto-contribution creation:** When a paycheck is saved, any deduction with a `contributionAccountType` (e.g. `401k`, `hsa`, `roth_401k`) automatically creates a corresponding contribution transaction in the transaction ledger. One entry, no duplication — the Contributions view stays current automatically.

Employer match contributions are tracked as separate contribution transactions (not paycheck deductions).

#### Contributions
A dedicated view aggregating contribution transactions by account type and year. Not a separate data store — purely derived from transactions where `isContribution: true`.

For each account type per year:
- YTD contribution total
- IRS annual limit (hardcoded per year, e.g. 401k: $23,500, IRA: $7,000, HSA: $4,300 for 2025)
- % of limit used
- Year-over-year comparison

Two contribution sources feed this view: paycheck auto-creation and direct contribution transactions.

#### Anti-Budget
Monthly anti-budget overview. Shows the pay-yourself-first math for any given month:

```
Income (from paychecks or income transactions)
− Savings targets (configured per month)
− Fixed costs (recurring expenses)
= Free money
```

`BudgetMonth` records hold the income and savings targets. Fixed costs are derived from transactions categorized as `fixed`. The free money figure is always informational — nothing in the app enforces it.

---

### Phase 3 — Forecasting

#### FIRE Forecasting
All projections run from FireProfile inputs — change any input and every projection updates.

- **Projected FI Date** — compound growth model: given current investable net worth, monthly contribution rate, and expected return, when does the portfolio hit the FIRE number?
- **Coast FIRE** — the investable net worth needed today so the portfolio grows to the FIRE number by target retirement age with zero additional contributions
- **Coast FIRE Date** — when the current trajectory reaches the Coast FIRE threshold
- **Lean FIRE variant** — same projections using a lower `annualExpensesTarget`
- **Fat FIRE variant** — same projections using a higher `annualExpensesTarget`
- **What-if scenario planner** — adjust savings rate, expected return, or target expenses and see the impact on projected FI date in real time

---

## Navigation

```
Dashboard · Accounts · Transactions · Paychecks · Contributions · Budget · Forecast · Settings
```

Settings houses FIRE Profile configuration and app preferences.

---

## Key Architectural Decisions

### Balance snapshots, not ledger-derived balances
Account balances are recorded directly as periodic snapshots rather than computed by summing transactions. This is simpler, more accurate for investment accounts (where gains/losses aren't transactions), and easier to backfill from brokerage statements.

### Anti-budget is a view, not a constraint
The budget module computes the anti-budget math; it never restricts what can be entered. No alerts, no locked categories, no "budget drift" to manage.

### Paychecks are standalone, not transactions
Paycheck records live in their own entity. They inform the anti-budget income figure and auto-generate contribution transactions, but do not produce income transactions themselves. This keeps the paycheck breakdown data clean and separate from general expense tracking.

### Paycheck deductions auto-create contributions
Deductions tagged with a `contributionAccountType` generate a contribution transaction on save. The Contributions view never needs manual input — recording a paycheck is sufficient for contribution tracking.

### Contributions are transactions + a smart view
Investment contribution deposits are regular transactions with `isContribution: true`. The Contributions page is a query, not a separate data store.

### Import layer is swappable
Both CSV import and future live sync providers (Teller.io for US banks, SimpleFIN Bridge as an alternative — both callable directly from a desktop app with the user's own tokens) feed the same transaction and paycheck stores. The `importSource` field is recorded for provenance but the rest of the app is agnostic to it.

### Local-first, no server
The app reads and writes a local libSQL file and runs fully offline. Cloud sync (Turso) is a background replica, not a dependency for normal use — if sync is unavailable the app still works, and reconciles later. This is what keeps it free to run and fast to use.

### FIRE calculations are profile-driven
All FIRE numbers, projections, and scenario variants derive from `FireProfile`. Lean and Fat FIRE are the same calculation with different `annualExpensesTarget` values — no separate data model needed.

---

## Data Model Summary

| Entity | Key Fields |
|---|---|
| `FireProfile` | Single row — also serves as app settings. `currentAge`, `targetRetirementAge`, `annualExpensesTarget`, `leanFireAnnualExpenses?`, `fatFireAnnualExpenses?`, `annualIncome`, `expectedReturnRate`, `inflationRate` |
| `Account` | `name`, `type`, `institution`, `isActive`, `includeInFireCalculations` |
| `AccountBalance` | `accountId`, `balance`, `recordedAt` |
| `Transaction` | `accountId`, `amount`, `description`, `date`, `type`, `category`, `isContribution`, `importSource` |
| `Paycheck` | `payDate`, `employer`, `payPeriod`, `grossAmount`, `netAmount`, `federalTax`, `stateTax`, `ficaTax`, `deductions[]`, `importSource` |
| `BudgetMonth` | `month`, `year`, `incomeTarget`, `savingsTarget` |

Contributions and net worth are derived views — no separate tables.

---

## Out of Scope (for now)

- Multi-user / multi-tenancy
- Live bank/brokerage sync (designed for, not built)
- PDF paycheck parsing
- Mobile app
- Email / push notifications
- Currency conversion / multi-currency
- App-level passcode / DB unlock UI (encryption-at-rest is delegated to OS full-disk encryption; an app-level passcode is a later addition)

## Migration Note

The repo was initially scaffolded as AdonisJS 7 + Inertia + Vue 3 + NuxtUI with auth. The Tauri pivot keeps the Vue/NuxtUI frontend and discards the Adonis server, Lucid ORM, Inertia, Tuyau, and the auth layer. Phase 1 implementation planning should account for standing up the Tauri shell and libSQL data layer before (or alongside) the first feature.
