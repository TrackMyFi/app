---
target: Contributions
total_score: 23
p0_count: 0
p1_count: 2
timestamp: 2026-06-20T12-04-57Z
slug: contributions
---
## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Progress bars show fill; estimated-limits alert is good. No loading skeleton on year change. |
| 2 | Match System / Real World | 3 | 401k/IRA/HSA/YTD/catch-up all domain-correct. Alert description contains a raw developer instruction exposed to users. |
| 3 | User Control and Freedom | 2 | Year selector is the only affordance. All transaction groups always expanded — no collapse, no escape from the wall of tables. |
| 4 | Consistency and Standards | 3 | Consistent card + table structure. `bg-amber-500` raw Tailwind mixes with `bg-success` semantic — violates CLAUDE.md rule. |
| 5 | Error Prevention | 2 | Over-limit (>100%) uses same amber as approaching-limit (80–99%). Exceeding IRS limits triggers a 6% excise tax — this needs a distinct error signal. |
| 6 | Recognition Rather Than Recall | 3 | Limits and percentages shown inline — nothing to memorize. Source labels present. |
| 7 | Flexibility and Efficiency | 2 | No collapse per group, no keyboard shortcuts, no direct link to associated accounts. |
| 8 | Aesthetic and Minimalist Design | 2 | YTD total, limit, and % appear in the card then again in the table group header. Most important aggregate number has least visual weight. |
| 9 | Error Recovery | 2 | Over-limit provides no guidance. Empty state has no actionable next step. |
| 10 | Help and Documentation | 1 | No tooltips on catch-up, no explanation of limit consequences, no contextual help. |
| **Total** | | **23/40** | **Acceptable — significant improvements needed** |

## Anti-Patterns Verdict

**LLM assessment:** Not egregiously AI-generated. Card grid is appropriate for contribution groups. The failure mode is the gap between "functional scaffold" and "crafted tool." The page buries its most important number and provides no compliance signal for over-limit states.

**Deterministic scan:** Clean — zero findings from the automated detector.

## Overall Impression

Solid bones: limit-tracking, YoY delta, catch-up amounts, correct domain language. But the most important number (total YTD) is styled smaller than individual card amounts, the same amber warning fires for "near limit" and "over limit" (a real compliance risk), and all transaction groups are always expanded creating a wall of tables.

## What's Working

1. **IRS limits surfaced inline** — catch-up amounts rendered correctly for 50+ users; a precise domain-correct detail many financial tools miss.
2. **Year-over-year delta** — "+/-$X vs. {year-1}" on each card answers "am I on pace?" without tab switching.
3. **Estimated-limits alert** — honest UAlert for projected limits; surfacing uncertainty rather than presenting guesses as facts.

## Priority Issues

**[P1] Over-limit uses the wrong color — and a raw Tailwind class**
- barColor() returns bg-amber-500 for both ≥80% (approaching) and >100% (exceeded). Exceeding IRS limits triggers a 6% excise tax. Same signal for "almost there" and "taxable problem." Also bg-amber-500 is a raw Tailwind class violating CLAUDE.md semantic rule.
- Fix: pctUsed > 1 → bg-error; pctUsed >= 0.8 → bg-warning; otherwise bg-success. Add "Over limit" text label when >1.
- Command: /impeccable harden

**[P1] YTD total has the lowest visual weight on the page**
- Total contributions displayed as text-sm inline text, smaller than the card amounts it summarizes.
- Design system: "The value always uses the display scale (24px/700) — never smaller."
- Fix: Replace stat row with a proper stat card or elevate to display-weight.
- Command: /impeccable layout

**[P2] Mono Numbers Rule violated in breakdown string**
- Breakdown renders as a .join() plain text string — currency values inside have no font-mono tabular-nums.
- Fix: Render breakdown items as individual <span> elements with font-mono.
- Command: /impeccable polish

**[P2] Progress bars convey status by color alone — no accessible signal**
- No role="progressbar", no aria-valuenow/min/max, no aria-label. h-1.5 (6px) bars barely perceivable.
- Fix: Add ARIA attributes. Increase to h-2.
- Command: /impeccable audit

**[P2] Empty state is a dead end**
- "No contributions recorded for 2026." with no next step or guidance.
- Fix: Add pointer to Paychecks/Transactions as sources of contribution data.
- Command: /impeccable onboard

## Persona Red Flags

**Alex (Power User):** All 5 transaction groups always expanded with no collapse — wall of tables. High abandonment risk for frequent check-ins.

**Jordan (First-Timer):** Alert description shows developer instruction ("Update irsLimits.ts...") to end users. "Catch-up contributions" has no tooltip. Trust erosion from three small confusions.

**Sam (Accessibility-dependent):** Progress bars have no ARIA roles/labels. text-xs text-muted (12px) may not meet 4.5:1 contrast. Color-only state distinction.

## Minor Observations

- h1 is font-semibold (600) — design system specifies Display at 700.
- barColor's >100% branch returns amber but so does the 80–99% branch; intended differentiation never implemented.
- Table group headers repeat YTD/Limit/% from cards above, but "%" alone lacks the "of $Y limit" context.
- "via Paycheck" vs "Manual" — asymmetric "via" prefix is unfinished labeling.
- yoyDelta red-codes any negative value; contributing less isn't always bad (text-muted or text-warning would be less alarmist).

## Questions to Consider

- "What if each card's progress bar expanded into a full-width visual on click, collapsing the transaction table beneath it rather than showing all groups at once?"
- "Should exceeding a limit surface a notification or badge elsewhere (Dashboard, nav item) rather than just an amber bar buried mid-page?"
- "What would this page look like if the primary question were 'Am I maxing all my limits?' instead of 'Here are my transactions organized by type?'"
