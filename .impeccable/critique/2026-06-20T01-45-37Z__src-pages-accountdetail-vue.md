---
target: AccountDetail
total_score: 22
p0_count: 0
p1_count: 3
timestamp: 2026-06-20T01-45-37Z
slug: src-pages-accountdetail-vue
---
## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | No loading states; no success feedback on save/add |
| 2 | Match System / Real World | 3 | Language is natural; "Danger Zone" is a recognized convention |
| 3 | User Control and Freedom | 3 | Cancel/confirm dialogs present; back nav exists |
| 4 | Consistency and Standards | 2 | Currency values inconsistently skip font-mono; balance text-size undifferentiated from label text-size |
| 5 | Error Prevention | 2 | Destructive confirmations exist; add-snapshot form has no validation |
| 6 | Recognition Rather Than Recall | 2 | Icon-only row actions; chart mode switch completely undiscoverable |
| 7 | Flexibility and Efficiency | 2 | No keyboard shortcuts; no bulk operations; daily-use app deserves power paths |
| 8 | Aesthetic and Minimalist Design | 3 | Clean layout, appropriate density; delta badges in two nested levels create some noise |
| 9 | Error Recovery | 2 | No API error surfacing; linked-transaction modal is functionally useless |
| 10 | Help and Documentation | 1 | No tooltips; no hint that opening a month updates the chart |
| **Total** | | **22/40** | **Acceptable — significant improvements needed** |

## Anti-Patterns Verdict

**LLM assessment**: No obvious AI slop. No gradient text, no side-stripe borders, no eyebrow-on-every-section, no glassmorphism. The design is disciplined and follows the system rules. The failure here isn't noise — it's absence. A FIRE tracker's account detail should lead with a signal ("you have $142,000 in this account") and the page never delivers that.

**Deterministic scan**: The detector returned zero findings (exit code 0). The file is clean against all automated pattern rules.

## Overall Impression

The page does the structural job correctly: it loads data, displays history, and lets you edit snapshots. But it buries its most important output. A user opens AccountDetail to know one thing — what is this account worth right now — and the page makes them infer it from a chart or dig into the first accordion row. That's the central miss.

## What's Working

1. **Accordion + chart interaction is genuinely clever.** When the user opens a month, the chart silently switches from monthly to intramonth view — a contextual mode shift that respects the user's current focus without requiring a separate control.

2. **Destructive operation safety is solid.** Archive and delete both use the Tauri confirm() dialog with specific account names and specific consequences in the copy.

3. **Inline editing inside the accordion is the right call.** Using a modal for snapshot edits would have added friction. The inline grid that flips between read and edit mode keeps the user in context.

## Priority Issues

**[P1] No current balance shown at the top**
- What: The page has no visible stat for the account's current balance.
- Why it matters: Users open account detail to get a number. Making them derive it from a chart or accordion summary fails the "Your numbers, your story" principle.
- Fix: Add a StatCard immediately below the account header with latestBalance from monthSummaries[0], label "Current Balance", hint "as of [date]".
- Suggested command: /impeccable layout

**[P1] Currency values skip font-mono**
- What: The accordion trailing slot balance and the snapshot row balance both display currency without font-mono.
- Why it matters: Misaligned numbers in a financial app undermine trust. The design system named this "The Mono-Numbers Rule" as non-optional.
- Fix: Add font-mono to both the trailing balance span and the snapshot body balance span.
- Suggested command: /impeccable polish

**[P1] Linked Transaction modal is a shipped placeholder**
- What: The linked transaction modal body is just "Transaction #{{ txnForModal }}" — tells the user nothing useful.
- Why it matters: A feature that appears to exist but does nothing is worse than not having the feature. The receipt icon creates expectation; the modal breaks it.
- Fix: Either (a) fetch transaction details and show amount/date/description, or (b) remove the receipt button entirely until the feature is complete.
- Suggested command: /impeccable harden

**[P2] Archive button has equal visual weight to Edit**
- What: The page header presents Edit and Archive side by side as co-equal actions. Archive is a semi-destructive, rarely-needed operation.
- Why it matters: Placing Archive in the primary action bar trains users to expect it frequently, and puts accidental misclick one tap from data loss.
- Fix: Move Archive into a UDropdownMenu on the Edit button, or as a link-style action in the Danger Zone section.
- Suggested command: /impeccable layout

**[P2] Chart mode switch is undiscoverable**
- What: When a user opens an accordion month, the chart changes from monthly to intramonth view with no affordance signaling this behavior exists.
- Why it matters: This is genuinely useful contextual behavior that most users will never find.
- Fix: Add a brief fade transition to the chart title when mode changes. Add a small hint line: "Open a month below to see daily snapshots."
- Suggested command: /impeccable animate

## Persona Red Flags

**Alex (Power User / FIRE user)**: Lands on account detail and has to read a chart to know the current balance — 3+ seconds of cognitive work before seeing the number. No keyboard shortcut for "Add Snapshot". The chart/accordion interaction is a delight once discovered and never taught.

**Riley (Stress Tester)**: Opens the linked transaction modal — sees "Transaction #47", a dead end. Adds a $0.00 snapshot — no validation, no warning. Adds a future-dated snapshot — succeeds silently. Two separate "No snapshots yet" empty states that are redundant.

**Sam (Accessibility-Dependent)**: Back button is a <button> not a link — misleads screen readers. Icon-only pencil/trash buttons have no aria-label. Inline FIRE icon in header subtitle has no aria-hidden.

## Minor Observations

- Back button should be RouterLink or <a> for semantics and middle-click support.
- Snapshot row balance uses text-xs — same size as the date label. Balance should be at least text-sm to visually differentiate.
- Chart has no color configuration for the line — Unovis may render blue instead of emerald. Verify.
- Month and snapshot-level delta badges create visual repetition — consider if both levels are necessary.
