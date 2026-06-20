---
target: Transactions
total_score: 25
p0_count: 0
p1_count: 2
timestamp: 2026-06-20T11-43-44Z
slug: src-pages-transactions-vue
---
## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | No loading indicator when month/filters change; toasts on save are good |
| 2 | Match System / Real World | 3 | Terminology maps well to FIRE literacy; "Savings Rate" lands naturally |
| 3 | User Control and Freedom | 2 | No undo for deletions; form has no Cancel button, only modal X |
| 4 | Consistency and Standards | 2 | Raw `<button>` for chart toggle vs. `UButton` everywhere else; hardcoded hex colors in chart vs. semantic tokens |
| 5 | Error Prevention | 2 | Delete is confirmed; form silently no-ops on missing account with zero feedback |
| 6 | Recognition Rather Than Recall | 3 | Visible filters and column headers; active filter state not shown |
| 7 | Flexibility and Efficiency | 2 | No keyboard shortcuts; requires "Apply" click after setting filters |
| 8 | Aesthetic and Minimalist Design | 3 | Clean at macro level; eyebrow pattern and gradient bars add noise |
| 9 | Error Recovery | 2 | No error UI in the form; delete has no undo |
| 10 | Help and Documentation | 2 | Balance toggle has a useful inline hint; otherwise no contextual help |
| **Total** | | **25/40** | **Acceptable — significant improvements available** |

## Anti-Patterns Verdict

**LLM assessment**: The macro aesthetic is clean but the `text-xs font-semibold uppercase tracking-widest text-muted` eyebrow appears five or more times across the page and child components. Chart title, monthly stat label, annual stat label, "Cumulative Net," "Income vs. Expense" — each section gets its own tracked all-caps kicker. That's the exact pattern in the absolute bans.

**Deterministic scan**: 2 findings in TransactionChart.vue — hardcoded #22c55e and #ef4444 bypass the token system (lines 96–97).

## Priority Issues

**[P1] Uppercase tracked eyebrows on every section header** — 5+ instances of `text-xs font-semibold uppercase tracking-widest text-muted` across Transactions.vue and TransactionChart.vue. Fix: drop eyebrows on stat cards; use `text-sm font-medium text-heading` or nothing.

**[P1] Hardcoded hex colors in TransactionChart bypass token system** — `barColors = ['#22c55e', '#ef4444']` and tooltip inline styles. Fix: read CSS variable at render time via getComputedStyle.

**[P2] Stats row is financial prose, not a metrics display** — income/expense/net/savings rate rendered as `flex flex-wrap` sentence fragments. Fix: 4-column horizontal layout per card, large value on top, muted label below.

**[P2] Raw `<button>` elements for chart mode toggle** — no focus ring, no design-system hover, inconsistent with UButton. Fix: replace with UTabs or UButton segmented toggle.

**[P2] Gradient decoration on category bars** — `bg-gradient-to-br from-info/75 to-info/50` on progress bars. Fix: flat `bg-info/60` fills.

## Minor Observations
- Transaction count `(N transactions)` is very faint (text-muted/75)
- Form has no Cancel button — only modal X
- Truncated descriptions have no tooltip to see full value
- 0-amount transactions save silently; same-account transfers not guarded
