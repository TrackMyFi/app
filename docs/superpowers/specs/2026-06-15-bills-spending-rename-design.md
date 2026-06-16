---
title: Rename Fixed/Discretionary Categories to Bills/Spending
date: 2026-06-15
status: approved
---

## Summary

Rename the two least-intuitive spending category display labels to reduce the cognitive overhead of categorizing transactions. The internal data values are unchanged — this is a pure UI label change.

## Problem

The current labels `Fixed` and `Discretionary` require users to know that:
- "Fixed" means *predictable/consistent amount*, not *non-optional*
- "Discretionary" is finance jargon for *variable or optional spending*

New users mis-categorize expenses (e.g. putting variable energy bills in "Discretionary" because the amount changes, or treating "Fixed" as "required/essential"). The correct mental model is not obvious from the labels alone.

## Solution

Rename the display labels:
- `Fixed` → `Bills`
- `Discretionary` → `Spending`

**Why these words:**
- "Bills" maps naturally to how people talk — mortgage, insurance, internet, utilities are all "bills" regardless of whether the amount varies slightly month to month.
- "Spending" covers day-to-day purchases (groceries, dining, gas, entertainment) where the total fluctuates based on choices.
- Together, they make the "can I cut this?" question intuitive: Bills are obligations; Spending is where you look first.

`Savings` and `Uncategorized` are left unchanged — they are already clear.

## Scope

### What changes
- Display labels in `CATEGORY_LABELS` (`src/lib/transactions/constants.ts`)
- Hardcoded category name strings in empty-state messages (`src/pages/Budget.vue`)

### What does NOT change
- Internal category values (`'fixed'`, `'discretionary'`) — no database migration required
- `CATEGORIES` array, `Category` type, or any store/API logic
- Any component that derives its display text from `categoryItems` or `labelForCategory` — these update automatically

### Affected files
| File | Change |
|------|--------|
| `src/lib/transactions/constants.ts` | Update `CATEGORY_LABELS`: `fixed → 'Bills'`, `discretionary → 'Spending'` |
| `src/pages/Budget.vue` | Update empty state strings for fixed and discretionary sections |

### Auto-updated (no direct edit needed)
All dropdowns in `TransactionForm.vue`, `ImportWizard.vue`, and the Budget page section headers consume `categoryItems` / `labelForCategory`, so they reflect the rename automatically.

## Out of Scope

- A "Required" or "Essential" 4th category (explored and deferred — the cuttability question is better addressed as a future "FIRE floor / lean budget" feature)
- Help text or tooltips (not needed given the clarity of the new labels)
- Any changes to how category rules, budgets, or forecasts work
