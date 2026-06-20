---
name: TrackMyFI
description: Local-first FIRE tracker for the long road to financial independence
colors:
  primary: "oklch(69.6% 0.17 162.48)"
  primary-deep: "oklch(59.6% 0.145 163.225)"
  primary-tint: "oklch(97.9% 0.021 166.113)"
  surface: "#ffffff"
  surface-muted: "oklch(98.7% 0.002 197.1)"
  surface-elevated: "oklch(96.3% 0.002 197.1)"
  surface-accented: "oklch(92.5% 0.005 214.3)"
  border: "oklch(92.5% 0.005 214.3)"
  border-accented: "oklch(87.2% 0.007 219.6)"
  text-body: "oklch(37.8% 0.015 216)"
  text-muted: "oklch(56% 0.021 213.5)"
  text-heading: "oklch(21.8% 0.008 223.9)"
  text-dimmed: "oklch(72.3% 0.014 214.4)"
typography:
  display:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.01em"
  title:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0.05em"
  mono:
    fontFamily: "ui-monospace, SFMono-Regular, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    fontFeature: '"tnum" 1'
rounded:
  xs: "2px"
  sm: "4px"
  md: "8px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "6px 14px"
  button-primary-hover:
    backgroundColor: "{colors.primary-deep}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "6px 14px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.text-body}"
    rounded: "{rounded.sm}"
    padding: "6px 14px"
  button-ghost-hover:
    backgroundColor: "{colors.surface-elevated}"
    textColor: "{colors.text-heading}"
    rounded: "{rounded.sm}"
    padding: "6px 14px"
  stat-card:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-body}"
    rounded: "{rounded.md}"
    padding: "16px"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.text-body}"
    rounded: "{rounded.sm}"
    padding: "8px 12px"
  nav-item-active:
    backgroundColor: "{colors.surface-elevated}"
    textColor: "{colors.text-heading}"
    rounded: "{rounded.sm}"
    padding: "8px 12px"
---

# Design System: TrackMyFI

## 1. Overview

**Creative North Star: "The Long Game"**

TrackMyFI is designed for people who measure time in decades, not quarters. The interface reflects this posture: calm, unhurried, precise. Every pixel on screen exists to help a person understand where they stand on a journey that might take twenty years. That requires restraint. Numbers carry the weight; decoration doesn't.

The system uses a cool mist neutral as its canvas — a blue-tinged gray that reads as the quiet of early morning, before the noise of the day starts. Emerald is the single voice of progress: sparing, earned, only there when something matters. Dark mode inverts the atmosphere to a deep mist-navy, the same quality but nocturnal — like checking your numbers late at night by lamplight.

This system explicitly rejects the spreadsheet aesthetic. The data here is structured, but the experience should never feel like an Excel sheet with a new color. It rejects gamified personal finance dashboards — no color-coded everywhere, no flashy charts built to impress. It rejects corporate fintech — stiff, navy-and-gold, zero personality. The design should feel like a tool someone made for themselves and uses every day.

**Key Characteristics:**
- Flat tonal surfaces — depth through layering, never shadows
- Emerald as a single earned accent, used sparingly
- Monospaced currency values — precision is part of the identity
- Compact but not cramped: 14px body, 12px labels, generous line height
- Warm but not soft: slight warmth from the emerald accent, cool canvas beneath

## 2. Colors: The Still Morning Palette

A cool, understated palette where emerald earns every appearance as a signal of progress or action.

### Primary
- **Steady Growth** (`oklch(69.6% 0.17 162.48)` — emerald-500): The primary brand color. Used for action buttons, active states, progress indicators, and key metrics that represent positive forward motion. Its green is lush but not shouting — a forest color, not a traffic-light color.
- **Deep Growth** (`oklch(59.6% 0.145 163.225)` — emerald-600): Hover and pressed state for primary actions. Also used for text links when emerald appears on a light background.
- **Growth Tint** (`oklch(97.9% 0.021 166.113)` — emerald-50): Subtle tinted backgrounds for success states, selected rows, and soft callouts. Use very sparingly — one tinted area per screen maximum.

### Neutral
- **Clean Canvas** (`#ffffff`): Page background. Never tinted warm.
- **Soft Mist** (`oklch(98.7% 0.002 197.1)` — mist-50): Muted/secondary backgrounds. Used for the `bg-muted` semantic slot.
- **Lifted Mist** (`oklch(96.3% 0.002 197.1)` — mist-100): Elevated surface backgrounds — sidebar, table headers, modal sub-sections. The `bg-elevated` slot.
- **Condensed Mist** (`oklch(92.5% 0.005 214.3)` — mist-200): Default border color and the `bg-accented` slot. The hairline that separates surfaces.
- **Clear Air** (`oklch(72.3% 0.014 214.4)` — mist-400): Dimmed/placeholder text. Hint text, empty states, optional labels.
- **Quiet Slate** (`oklch(56% 0.021 213.5)` — mist-500): Secondary text. Section labels, account types, supporting metadata.
- **Deep Slate** (`oklch(37.8% 0.015 216)` — mist-700): Primary body text. Everything the user reads as content.
- **Near Black** (`oklch(21.8% 0.008 223.9)` — mist-900): Headings and highlighted text. Page titles, account names, critical values.

### Named Rules
**The One Voice Rule.** Emerald is the only accent color in the system. It is used for primary buttons, active nav items, progress, and success — and nowhere else. When everything is green, nothing matters. Reserve emerald for moments that tell the user "this is moving forward."

**The Cool Canvas Rule.** Page backgrounds are white or cool-tinted mist. Never warm-tinted. Warmth in this system comes from emerald and spacing rhythm, not from tinting the canvas beige.

## 3. Typography

**Body Font:** system-ui, -apple-system, sans-serif (system default — renders as SF Pro on macOS, Segoe UI on Windows)

**Mono Font:** ui-monospace, SFMono-Regular, monospace (system monospace — SF Mono on macOS)

**Character:** Single-family system type at a range of weights. The decision to use system fonts is intentional: this is a local desktop app that should feel native, not a web product trying to express personality through font licensing. Precision comes from weight, size hierarchy, and monospaced numerals — not from an expressive display typeface.

### Hierarchy
- **Display** (700 weight, 1.5rem/24px, line-height 1.25, letter-spacing -0.01em): Page-level headings (Dashboard, Accounts, Transactions). One per screen.
- **Title** (600 weight, 1.125rem/18px, line-height 1.4): Section headings within a page (Net Worth Over Time, FIRE Accounts). Sparingly — most sections need a label, not a heading.
- **Body** (400 weight, 0.875rem/14px, line-height 1.5): All body content — table rows, card descriptions, form labels. The workhorse.
- **Label** (600 weight, 0.75rem/12px, line-height 1.4, letter-spacing 0.05em, uppercase): Section dividers and column headers. `FIRE ACCOUNTS`, `TYPE`, `BALANCE`. This is deliberate sparse scaffolding, not decoration.
- **Mono** (400 weight, 0.875rem/14px, tabular numbers `tnum`): All currency values and numeric data. Aligns columns, respects precision. Always used for financial numbers.

### Named Rules
**The Mono-Numbers Rule.** Every currency value, percentage, and financial metric is rendered in `font-mono` with `font-variant-numeric: tabular-nums`. Numbers that don't align are untrustworthy. This is not optional.

**The Label Ceiling Rule.** Uppercase tracked labels (`text-xs font-semibold uppercase tracking-wider`) are used only for column headers and section dividers where hierarchy demands it — not as eyebrows above every section. One or two per screen maximum.

## 4. Elevation

This system is flat by default. Depth is conveyed through tonal layering — surfaces stack via background color, not shadow. The NuxtUI semantic slots (`bg`, `bg-muted`, `bg-elevated`, `bg-accented`) define three tonal levels:

1. **Canvas** (white / mist-900 dark): The page itself.
2. **Elevated** (mist-100 / mist-800 dark): Sidebar, table header rows, card backgrounds, modal sub-areas. One step above canvas.
3. **Accented** (mist-200 / mist-700 dark): Interactive hover states, selected backgrounds, deeper nesting.

Borders (`mist-200`) define separation within a tonal level — between table rows, around cards, along the sidebar edge.

### Named Rules
**The Flat-First Rule.** No box shadows on cards, panels, or modals at rest. If something needs visual separation, use tonal background or a border. Shadows are never decorative and never ambient. The rare exception: a dropdown or tooltip that must float above other content uses a minimal system-level shadow, not a designed one.

## 5. Components

### Buttons
- **Shape:** Gently rounded (4px radius — `rounded`, one full step above square)
- **Primary:** Emerald-500 background, white text, 6px/14px padding. Icon-left at 16px. Transitions to emerald-600 on hover (150ms ease-out).
- **Focus:** `outline-primary/25` — a faint emerald ring, 3px at focus-visible, no offset.
- **Ghost / Neutral:** Transparent background, mist-700 text. On hover, lifts to `bg-elevated`. Used for secondary actions and icon-only controls.
- **Destructive:** `color="error"` slot — renders in NuxtUI error red. Only in destructive dropdown menu items, never as a primary CTA.
- **Size:** Default `md` with 16px icons. `xs` for inline icon-only actions (dropdown triggers, edit buttons within table rows).

### Stat Cards
The signature component. Label (12px/500/muted) over value (24px/600/heading) with an optional hint line (12px/muted italic). Wrapped in a `UCard` — rounded-lg (8px), border-default, white background, 16px padding.

- **Do not** put progress bars, trend arrows, or sparklines inside stat cards unless the design specifically calls for them. The number is enough.
- The value always uses the display scale (24px/700) — never smaller.

### Cards / Containers
- **Corner Style:** Gently curved (8px radius — `rounded-lg`)
- **Background:** White (`bg-default`), elevated sections use `bg-elevated`
- **Shadow Strategy:** None. Border `border-default` (mist-200) defines the boundary.
- **Internal Padding:** 16px (`p-4`) standard; 24px (`p-6`) for chart containers and full-page sections.

### Tables / List Rows
Account lists, transaction tables, and similar structured data use a hand-rolled grid pattern rather than a generic table. Key rules:
- Table headers: `bg-elevated` background, 12px/600/uppercase/muted labels, `border-b border-default`
- Data rows: White background, `border-b border-default`, `hover:bg-elevated/50` on interactive rows
- Right-aligned numeric columns with `font-mono`
- `text-sm` for data content, never smaller

### Inputs / Fields
- **Style:** Border-default stroke, white background, rounded-sm (4px)
- **Focus:** Primary outline ring (`outline-primary/25`)
- **Currency / Numeric:** `CurrencyInput` and `PercentInput` components — monospaced font, right-aligned
- **Error:** `border-error` with `text-error` helper text below

### Navigation
- **Style:** 224px sidebar (`w-56`), `border-r border-default`, 12px padding
- **Header:** Logo icon (24px) + "TrackMyFI" in 14px/600/tight-tracking. 12px padding all sides.
- **Items:** 14px/400, `rounded` (4px), full-width. Default: transparent bg, mist-700 text. Active: `bg-elevated`, mist-900 text. Hover: `hover:bg-elevated`. Icon at 16px left of label.
- No collapse, no nesting, no mobile hamburger — desktop-native, always visible.

### Charts
Built with Unovis. Line charts for net worth over time and account balance history. Key visual rules:
- Line color: emerald (`primary`)
- Area fill: emerald-tint (`primary-tint`) at low opacity
- Grid lines: mist-200, very subtle
- No chart borders or card shadows — charts live in a padded container section
- Tooltips use `bg-elevated` background with `border-default` border

## 6. Do's and Don'ts

### Do:
- **Do** use `font-mono` with tabular numbers for every currency value and financial metric — alignment is a trust signal.
- **Do** use emerald only for primary buttons, active states, progress indicators, and success signals. One voice.
- **Do** use tonal layering (bg → bg-elevated → bg-accented) to create depth. Borders separate; backgrounds layer.
- **Do** give page titles the full display weight (24px/700). Financial data needs clear hierarchy anchors.
- **Do** use uppercase tracked labels (`text-xs font-semibold uppercase tracking-wider`) for table column headers and section dividers — and only there.
- **Do** keep the sidebar simple and static — no collapsing, no nested items, no badges unless earned.

### Don't:
- **Don't** build dense tabular interfaces that look like Excel reskinned. Every column, every row must earn its place. If it looks like a spreadsheet, it's failing.
- **Don't** use shadows on cards, modals, or panels at rest. This system is flat. Shadows belong only where strict floating is required (dropdowns, tooltips).
- **Don't** add color-coded everything — green for gains, red for losses, yellow for warnings covering every row. Color should be signal, not decoration. Use it where it changes a decision.
- **Don't** use gradient text, glassmorphism, or gradient backgrounds. The palette is clean and precise; gradients add noise without adding meaning.
- **Don't** put emerald on secondary or tertiary elements. Emerald's meaning comes from its rarity — buttons, active items, progress, success only.
- **Don't** use `border-left` as a colored accent stripe on cards, list items, or callouts. Use a full border, a background tint, or nothing.
- **Don't** make the design noisy with too many states, hover effects, and animated elements competing for attention. This is a tool the user opens to check numbers and close.
