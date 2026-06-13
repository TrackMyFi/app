# Phase 3 — Forecasting: Design Spec

**Date:** 2026-06-13
**Status:** Approved
**Builds on:** `docs/superpowers/specs/2026-06-09-trackmyfi-design.md` (Phase 3 — Forecasting)

## Summary

Phase 3 adds FIRE forecasting to TrackMyFI: Coast FIRE, Lean/Fat FIRE variants, a
required-contribution back-solve, and a live what-if scenario planner — surfaced on a new
**Forecast** page. All math is pure, unit-tested TypeScript in `src/lib/fire/`. **No new
database tables and no migrations.** Everything derives from `FireProfile`, the existing
accounts/balances data, and the existing contribution transactions.

This phase also replaces the Phase 1 crude `savingsRate` approximation with a real
contribution-derived monthly figure, and reconciles the Dashboard to use it.

## Decisions (from brainstorming)

1. **Monthly contribution baseline:** derive from actual data (trailing-12-month average of
   real contribution transactions), and allow override in the what-if planner. The override is
   the scenario — it is **not** saved to the profile. (Question 1 → C)
2. **What-if persistence:** ephemeral. Sliders reset to baseline each visit; nothing persists.
   No new tables. (Question 2 → A)
3. **Metric set per target:** FIRE number, Projected FI date, Years to FI, Coast FIRE number,
   Coast FIRE status, **and** Required monthly contribution to hit `targetRetirementAge`.
   (Question 3)
4. **Coast anchor:** profile `targetRetirementAge`. (Question 3)
5. **Layout:** three variant cards (Lean / Regular / Fat) side by side, Regular primary, with
   the what-if sliders tucked into a **right-side drawer** (not always-on). A projection chart
   sits above the cards. (Layout A + drawer)
6. **Projection chart:** include one. Regular-FIRE growth curve from today, horizontal
   FIRE-number reference line, Coast-threshold marker; updates live with the sliders.
   (Question 4 → A)
7. **Dashboard reconciliation:** update Dashboard to use the new derived contribution; drop the
   "Approximate" hint on Savings Rate. (Question 5 → A)

## Architecture

Consistent with established conventions (see `project_trackmyfi_design` memory): all new math is
**pure TypeScript in `src/lib/fire/`, fully unit-tested**. No new Rust types (ts-rs unaffected),
no migrations.

### New / changed modules

| Module | Responsibility |
|---|---|
| `src/lib/fire/coast.ts` | Coast FIRE number, coast status, coast-crossing date |
| `src/lib/fire/requiredContribution.ts` | Back-solve: monthly contribution to hit the FIRE number by `targetRetirementAge` |
| `src/lib/fire/contributionRate.ts` | Derived monthly contribution baseline (trailing-12-mo average of real contributions), with `savingsRate` estimate as the `<12 months` fallback |
| `src/lib/fire/forecast.ts` | `buildForecast(...)` aggregator → full Lean/Regular/Fat × 6-metric result object; applies what-if overrides |
| `src/lib/fire/projection.ts` (extend) | Add `projectionSeries(...)` emitting growth-curve points for the chart |
| `src/lib/fire/index.ts` (extend) | Re-export new functions |
| `src/pages/Forecast.vue` | New page (chart + variant cards + what-if drawer) |
| `src/pages/Dashboard.vue` (edit) | Use derived contribution; drop "Approximate" hint |
| router + nav | New `Forecast` route + nav link |

**Derived-contribution data source:** reuse Phase 2c's existing
`list_contribution_txns_cmd(year)` Rust command (queries selected + prior year). A small TS
helper sums the trailing 12 months from those rows. **No new Rust command.**

## The math (six metrics, per expense target)

For each of **Lean / Regular / Fat** — `expenses` falls back to `annualExpensesTarget` when the
lean/fat field is unset:

1. **FIRE number** = `expenses × 25` (existing `fireNumber`).
2. **Projected FI date** = existing `projectedFiDate(investable, monthly, return, inflation, fireNumber)`.
3. **Years to FI** = months-to-FI ÷ 12 (derived from the existing `monthsToFire`).
4. **Coast FIRE number** = `fireNumber ÷ (1 + realReturn)^yearsToRetirement`, where
   `yearsToRetirement = targetRetirementAge − currentAge`, using the existing real-return basis
   (`realMonthlyReturn` compounded, or the annual real-return equivalent). This is the present
   value that compounds to the FIRE number by retirement with **zero** further contributions.
5. **Coast status** = if `investable ≥ coastNumber` → "Coasting" (true); else the date the
   current trajectory (with contributions) crosses the coast threshold — solved with the same
   iterative `monthsToFire` loop, target = coast number.
6. **Required monthly contribution** = back-solved annuity payment so `investable` grows to the
   FIRE number in exactly `yearsToRetirement`. Closed form:
   `PMT = (FV − PV·(1+r)^n) · r / ((1+r)^n − 1)`, with monthly real `r` and `n = months to
   retirement`. Returns 0 / "already there" when `investable` already meets/exceeds the target.

All functions return `null` gracefully when a target is unreachable within the 100-year cap,
matching `monthsToFire`'s existing contract. `yearsToRetirement ≤ 0` (already at/past retirement
age) is handled explicitly (no division by zero; coast number = FIRE number, required = n/a).

## Forecast page UI

New route + nav link, slotted before **Settings** per the design doc nav order
(`Dashboard · Accounts · Transactions · Paychecks · Contributions · Budget · Forecast · Settings`).

**Default view (drawer closed):**
- Header with title + `⚙ What-if` button (top-right).
- **Projection chart** (unovis, following `NetWorthChart` as the pattern): Regular-FIRE growth
  curve from today, horizontal FIRE-number reference line, Coast-threshold marker, FI-date
  crossing annotated.
- Three **variant cards** — Lean / Regular / Fat side by side, Regular visually primary — each
  showing the six metrics. Coast status renders as a green "Coasting ✓" badge or a date.

**What-if drawer (right-side):**
- Four sliders, initialized to the derived baseline: monthly contribution (derived 12-mo
  average), expected return, inflation, retirement age.
- Touching any slider enters **scenario mode**: a "Scenario — not saved · Reset" banner appears;
  every card and the chart recompute live via `buildForecast(..., overrides)`. **Reset** restores
  baseline. Ephemeral — closing the drawer or leaving the page discards overrides.

**State:** page-local component state (baseline + overrides). No Pinia store — the scenario is
ephemeral and not shared across pages. (Reuses `useFireProfileStore` and `useAccountsStore` for
inputs, plus the contribution query for the baseline.)

## Dashboard reconciliation

- Replace the crude `savingsRate`-derived monthly figure with the new derived monthly
  contribution from `contributionRate.ts`.
- "Projected FI Date" uses that figure.
- "Savings Rate" card: compute from real contributions ÷ income; **drop the
  "Approximate — refined in Phase 2" hint.**
- The old `savingsRate` function in `projection.ts` remains only as the documented `<12 months`
  fallback inside `contributionRate.ts`.

## Testing

- **Unit (Vitest):** `coast.test.ts`, `requiredContribution.test.ts`, `contributionRate.test.ts`,
  `forecast.test.ts` — known-input/known-output cases including edge cases: already-coasting,
  target unreachable, zero/negative contribution, lean/fat unset → fallback, `<12`-month data →
  estimate fallback, `yearsToRetirement ≤ 0`. Extend `projection.test.ts` for the series helper.
- **Type/build gates:** `vue-tsc`, `vite build`, `cargo build` stay green. ts-rs unaffected.
- **Manual GUI smoke test:** walk the Forecast page, drawer, sliders, reset; confirm Dashboard
  numbers match the Forecast Regular card.

## Out of scope (Phase 4+)

Saved/named scenarios; Monte Carlo / sequence-of-returns risk; tax-aware withdrawal modeling;
social-security or pension inputs.
