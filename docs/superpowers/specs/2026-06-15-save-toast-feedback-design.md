# Save Toast Feedback

**Date:** 2026-06-15  
**Status:** Approved

## Problem

Inline save actions throughout the app (Settings, Budget, Account, Balance, Paycheck, Transaction forms) give zero feedback after a successful write. The user has no confirmation the action worked.

## Approach

Add a `useToast().add(...)` success call at the end of each save function, after the async operation resolves. Nuxt UI v4's `useToast` composable is already available and `<UApp>` in `App.vue` handles rendering — no new dependencies or layout changes needed.

## Scope

Only success toasts. No error toasts, no loading state changes, no form logic refactoring.

## Toast Spec

All toasts use `color: 'success'` and auto-dismiss at the Nuxt UI default (~5 seconds). No description line. No icon override.

| File | Function | Condition | Title |
|---|---|---|---|
| `src/pages/Settings.vue` | `onSubmit` | always | "Profile updated" |
| `src/pages/Budget.vue` | `saveTarget` | always | "Budget target saved" |
| `src/components/AccountForm.vue` | `onSubmit` | create path | "Account created" |
| `src/components/AccountForm.vue` | `onSubmit` | update path | "Account updated" |
| `src/components/BalanceForm.vue` | `onSubmit` | always | "Balance recorded" |
| `src/components/BalanceRow.vue` | `save` | always | "Balance updated" |
| `src/components/PaycheckForm.vue` | `save` | create path | "Paycheck added" |
| `src/components/PaycheckForm.vue` | `save` | update path | "Paycheck updated" |
| `src/components/TransactionForm.vue` | `save` | create path | "Transaction added" |
| `src/components/TransactionForm.vue` | `save` | update path | "Transaction updated" |

## Implementation Notes

- Each file gets one `const toast = useToast()` at the top of `<script setup>`
- Toast call goes immediately after the awaited store method returns, before any `emit` or navigation
- No changes to error paths, loading states, or form structure
