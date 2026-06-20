---
target: Forecast
total_score: 22
p0_count: 0
p1_count: 2
timestamp: 2026-06-20T12-18-59Z
slug: src-pages-forecast-vue
---
## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Scenario banner is solid; no loading states while profile/accounts load |
| 2 | Match System / Real World | 3 | Audience is FIRE-literate, but "Required / mo" is too abbreviated |
| 3 | User Control and Freedom | 3 | Reset is present; scenarios can only be reset all-at-once, not per-slider |
| 4 | Consistency and Standards | 2 | Chart uses indigo #6366f1 — wrong color family entirely; currency values lack font-mono |
| 5 | Error Prevention | 2 | Slider ranges prevent bad values; zero contribution + negative real return edge cases unhandled |
| 6 | Recognition Rather Than Recall | 2 | Slider labels show current value but no min/max endpoints; "Required / mo" is cryptic |
| 7 | Flexibility and Efficiency | 2 | Sliders only — no direct numeric input; no saved/named scenarios; no comparison mode |
| 8 | Aesthetic and Minimalist Design | 3 | Page is clean; FI date buried in a dd; chart Y-axis shows raw unformatted numbers |
| 9 | Error Recovery | 1 | Blank page if profile not configured — no empty state, no guidance |
| 10 | Help and Documentation | 1 | No tooltips on "Coast number", "Coast status", "Required / mo"; terms assumed known |
| **Total** | | **22/40** | **Acceptable — significant improvements needed** |

## Anti-Patterns Verdict
LLM assessment: Not overtly AI-generated. Layout is uncluttered and the what-if drawer is a considered interaction. But the three FIRE variant cards — identical structure, same five fields, differentiated only by a ring on the middle one — read as a generic comparison grid. Chart colors break the design language: indigo-violet portfolio line in a system with exactly one accent (emerald).

Deterministic scan: 6 advisory findings — 3 in Forecast.vue:119-121 (chart legend swatches), 3 in ForecastChart.vue:36-38 (Unovis line colors). Colors: #6366f1 (indigo), #22c55e (Tailwind green-500), #f59e0b (amber). None are in the design system palette. Not false positives.

## Overall Impression
Strong structure undermined by chart color drift, missing font-mono on all financial values, and a buried FI date that deserves visual hero treatment. The most personally significant page in the app doesn't treat its most significant moment with proportional weight.

## What's Working
1. The What-if slideover pattern — non-blocking, live-updating, right choice over a modal.
2. Three-variant comparison (Lean/Regular/Fat FIRE) — legitimate insight surfaced at a glance.
3. Scenario warning banner — amber/warning, inline reset, self-dismissing. Exactly right.

## Priority Issues

[P1] Chart colors break the design system entirely
Portfolio line is #6366f1 (indigo) — not in DESIGN.md. FIRE number is #22c55e (different from system emerald). Coast is #f59e0b (amber). The system has one accent: emerald. Fix: use system primary for portfolio line, primary-deep for FIRE target, muted tone for coast. Extract to shared constants.

[P1] Currency values missing font-mono
Every dd in the variant cards and every value in the slideover renders in system-ui. DESIGN.md calls this non-optional: "Every currency value, percentage, and financial metric is rendered in font-mono." Without it, numbers don't align across rows when digit counts differ.

[P2] FI date is buried — page has no hero
Projected FIRE date is a dd inside a dl inside a UCard, identical visual weight to "Coast number." The FIRE number itself is text-xl (20px/600) — below stat-card display spec (24px/700). PRODUCT.md: "a projected FI date isn't just a number, it's a life target." Fix: hero row with FI date in display-scale type above or at the chart. FIRE number to text-2xl font-bold.

[P2] Chart Y-axis is unformatted
VisAxis type="y" has no tick-format. Unovis renders raw ticks: 500000, 1000000. Fix: compact currency formatter — $1.2M, $500K.

[P2] Empty state: blank page when profile isn't configured
If fp.profile is null, forecasts is empty, the page shows nothing. No guidance. Fix: v-if guard with empty state pointing to Settings to configure FIRE profile.

## Persona Red Flags
Alex: can't type a specific contribution amount — slider only. Can't save/compare scenarios. FI date update is occluded by the open slideover.
Sam: Chart legend conveys series identity through color alone (style="border-color:#6366f1") with text labels but no shape/pattern differentiation. No ARIA live region announcing downstream FI date changes when sliders move.
Riley: Inflation >= return rate → negative real return — does buildForecast handle this? Required monthly could go negative or astronomically large at extreme slider values with no cap or error state.

## Minor Observations
- "Required / mo" is the most abbreviated label; needs expansion or tooltip.
- Baseline estimated caveat should be near the Monthly contribution slider, not at the bottom.
- Card label is text-sm (14px), should be text-xs (12px) per DESIGN.md Label spec.
- coastText returns emoji ✓ — inconsistent with Phosphor icon system.
