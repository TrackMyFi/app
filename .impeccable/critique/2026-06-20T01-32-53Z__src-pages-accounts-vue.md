---
target: Accounts
total_score: 25
p0_count: 0
p1_count: 1
p2_count: 4
timestamp: 2026-06-20T01-32-53Z
slug: src-pages-accounts-vue
---
## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | No loading indicator during initial fetch; archive/edit actions complete silently with no confirmation |
| 2 | Match System / Real World | 3 | FIRE terminology appropriate for audience; "FIRE ✓" label in detail page is slightly informal |
| 3 | User Control and Freedom | 3 | Archive is reversible, delete has confirmation; no undo within session, no bulk actions |
| 4 | Consistency and Standards | 3 | Unicode ▼/▶ triangles in archived toggle clash with Phosphor icons used everywhere else |
| 5 | Error Prevention | 3 | Delete requires confirmation; archive is reversible; no input on list page to validate |
| 6 | Recognition Rather Than Recall | 2 | Every account action (Edit, Archive, Delete) is hidden behind an unlabeled ⋯ button — zero affordance |
| 7 | Flexibility and Efficiency | 2 | No sorting, no filtering, no keyboard nav on rows, no batch actions |
| 8 | Aesthetic and Minimalist Design | 3 | Clean overall; 6 instances of the uppercase tracked label pattern violates the design system's 1–2 ceiling |
| 9 | Error Recovery | 2 | No error states for failed store loads; if Turso sync fails, the page silently shows nothing |
| 10 | Help and Documentation | 1 | No empty state, no tooltips, no guidance anywhere — entirely relies on FIRE fluency |
| **Total** | | **25/40** | **Acceptable — significant improvements available** |

## Anti-Patterns Verdict

**LLM assessment:** The interface does not read as AI-generated. Follows design system with real discipline — monospaced financials, tonal layering without shadows, emerald reserved for primary actions only. No gradient text, glassmorphism, or identical card grids. The main aesthetic risk is the uppercase label repetition, which is a design-system violation, not an AI tell.

**Deterministic scan:** Exit code 0 — clean. No rule violations detected in src/pages/Accounts.vue.

## Overall Impression

The bones are solid — feels like a native desktop tool, not a reskinned SaaS dashboard. The primary gap is information density at the wrong layer: the list page is where a FIRE user lands daily, but it hides all actions, shows no recency signal on balances, and offers nothing to a new user.

## What's Working

1. Monospaced, right-aligned balances — trust signal applied consistently.
2. Tonal layering without shadows — correct implementation of the flat system.
3. Stat cards as an anchor — Net Worth / FIRE / Non-FIRE give instant orientation.

## Priority Issues

**[P1] Missing empty state**
No guidance, no prompt, no visual affordance when user has no accounts. Three $0 stat cards and nothing else. Fix: inline empty state with icon + guidance + link to add form. Show — instead of $0 in stat cards.

**[P2] All account actions invisible behind unlabeled ⋯**
Edit, Archive, Delete hidden behind a button with no aria-label or title. Recognition over recall is broken. Fix: add aria-label="Account options", consider surfacing Edit inline on row hover.

**[P2] 6 instances of uppercase label pattern — design system violation**
FIRE ACCOUNTS, NON-FIRE ACCOUNTS, ARCHIVED section headers + 3 sets of column headers = 6 total. DESIGN.md limits to 1–2 per screen. Fix: remove uppercase section labels, use simple muted text or hairline rules instead. Keep column headers.

**[P2] Balances show no recency signal**
Latest balance with no date — can't tell if it's fresh or 6 months stale. Fix: show snapshot date in text-xs text-muted below balance; highlight accounts not updated in >30 days.

**[P2] Archived toggle uses raw unicode triangles**
▼/▶ glyphs instead of i-ph-caret-down/i-ph-caret-right. Missing aria-expanded. Fix: use UIcon with Phosphor caret, add :aria-expanded="showArchived".

## Persona Red Flags

**Alex (Power User):** Accounts in creation order, no sort/filter. No keyboard row navigation. No quick snapshot add from list. No batch actions.

**Sam (Accessibility):** ⋯ button has no accessible name. Archived toggle missing aria-expanded. Row divs are not keyboard-focusable (click handler on non-interactive element).

**Recurring FIRE Tracker (project-specific):** No quick-add snapshot from list. No visual indicator of stale balances. 24+ clicks for weekly update across 8 accounts.

## Minor Observations

- AccountDetail "← Accounts" uses unicode arrow + button-as-link. Should use i-ph-arrow-left icon.
- Linked Transaction modal shows only "Transaction #123" — unfinished placeholder.
- "FIRE ✓" in AccountDetail subtitle uses raw checkmark. Should use i-ph-check-circle in text-success.
- Three-column stat grid cramped at narrow desktop widths.
- AccountDetail accordion uses rounded-xl; system standard is rounded-lg.
