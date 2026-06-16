# Bills/Spending Category Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the `Fixed` and `Discretionary` category display labels to `Bills` and `Spending` to make transaction categorization immediately intuitive.

**Architecture:** Two targeted edits — update `CATEGORY_LABELS` in the constants file (single source of truth for display labels), and update hardcoded empty-state strings in the Budget page. All dropdowns and section headers derive their text from `categoryItems`/`labelForCategory`, so they update automatically. No database migration needed — internal values `'fixed'` and `'discretionary'` are unchanged.

**Tech Stack:** TypeScript, Vue 3, Vitest

---

### Task 1: Update CATEGORY_LABELS in constants.ts

**Files:**
- Modify: `src/lib/transactions/constants.ts:21-26`

- [ ] **Step 1: Update the two label values**

In `src/lib/transactions/constants.ts`, change lines 23–24:

```ts
export const CATEGORY_LABELS: Record<Category, string> = {
  savings: 'Savings',
  fixed: 'Bills',
  discretionary: 'Spending',
  uncategorized: 'Uncategorized',
}
```

- [ ] **Step 2: Run the test suite to confirm nothing broke**

```bash
npm test
```

Expected: all tests pass. The tests use internal values (`'fixed'`, `'discretionary'`), not display labels, so no test changes are needed.

- [ ] **Step 3: Commit**

```bash
git add src/lib/transactions/constants.ts
git commit -m "feat: rename Fixed→Bills and Discretionary→Spending category labels"
```

---

### Task 2: Update empty-state strings in Budget.vue

**Files:**
- Modify: `src/pages/Budget.vue:77-78`

- [ ] **Step 1: Update the two empty-state messages**

In `src/pages/Budget.vue`, find the `emptyMessage` function (around line 75) and update the two strings:

```ts
case 'fixed': return 'No bills this month.'
case 'discretionary': return 'No spending this month.'
```

- [ ] **Step 2: Run the test suite**

```bash
npm test
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/pages/Budget.vue
git commit -m "feat: update Budget empty-state messages to match Bills/Spending rename"
```
