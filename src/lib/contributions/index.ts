import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'
import type { YearLimits } from './irsLimits'

export interface ContributionBreakdown {
  type: string
  label: string
  total: number
}

export interface ContributionRow {
  label: string
  accountTypes: string[]
  total: number
  breakdown?: ContributionBreakdown[]
  limit?: number
  limitBase?: number
  catchUpAmount?: number
  pctUsed?: number
  yoyDelta?: number
}

interface GroupDef {
  label: string
  types: string[]
  order: number
  limitFor?: (
    limits: YearLimits,
    age: number,
    hsaCoverage: 'self' | 'family',
  ) => { base: number; catchUp?: number }
}

const TYPE_LABELS: Record<string, string> = {
  '401k': '401k',
  roth_401k: 'Roth 401k',
  traditional_ira: 'Traditional IRA',
  roth_ira: 'Roth IRA',
  hsa: 'HSA',
  brokerage: 'Brokerage',
  crypto: 'Crypto',
}

// Group definitions in display order. Limited types first, then unlimited.
const GROUPS: GroupDef[] = [
  {
    label: '401k / Roth 401k',
    types: ['401k', 'roth_401k'],
    order: 0,
    limitFor: (l, age) => ({
      base: l.k401,
      catchUp: age >= l.k401CatchUpAge ? l.k401CatchUp : undefined,
    }),
  },
  {
    label: 'Traditional / Roth IRA',
    types: ['traditional_ira', 'roth_ira'],
    order: 1,
    limitFor: (l, age) => ({
      base: l.ira,
      catchUp: age >= l.iraCatchUpAge ? l.iraCatchUp : undefined,
    }),
  },
  {
    label: 'HSA',
    types: ['hsa'],
    order: 2,
    limitFor: (l, age, hsaCoverage) => ({
      base: hsaCoverage === 'family' ? l.hsaFamily : l.hsaSelf,
      catchUp: age >= l.hsaCatchUpAge ? l.hsaCatchUp : undefined,
    }),
  },
  { label: 'Brokerage', types: ['brokerage'], order: 3 },
  { label: 'Crypto', types: ['crypto'], order: 4 },
]

function sumByType(
  txns: Transaction[],
  accounts: Account[],
  yearPrefix: string,
): Map<string, number> {
  const typeOf = new Map(accounts.map((a) => [a.id, a.type]))
  const out = new Map<string, number>()
  for (const t of txns) {
    if (!t.date.startsWith(yearPrefix)) continue
    const effectiveId = t.transferAccountId ?? t.accountId
    const type = typeOf.get(effectiveId)
    if (!type) continue // orphan txn (account deleted) — excluded
    // Amounts are summed signed: a negative (refund/correction) contribution
    // legitimately nets the type total down.
    out.set(type, (out.get(type) ?? 0) + t.amount)
  }
  return out
}

export function buildContributionRows(
  txns: Transaction[],
  accounts: Account[],
  year: number,
  age: number,
  hsaCoverage: 'self' | 'family',
  limits: YearLimits,
): ContributionRow[] {
  const thisYear = sumByType(txns, accounts, String(year))
  const priorYear = sumByType(txns, accounts, String(year - 1))

  const rows: (ContributionRow & { _order: number })[] = []

  for (const group of GROUPS) {
    const total = group.types.reduce((s, t) => s + (thisYear.get(t) ?? 0), 0)
    const priorTotal = group.types.reduce((s, t) => s + (priorYear.get(t) ?? 0), 0)

    // Omit groups with no contributions in either year.
    if (total === 0 && priorTotal === 0) continue

    const row: ContributionRow & { _order: number } = {
      label: group.label,
      accountTypes: group.types,
      total,
      yoyDelta: total - priorTotal,
      _order: group.order,
    }

    // Breakdown only for merged (multi-type) groups.
    if (group.types.length > 1) {
      row.breakdown = group.types.map((t) => ({
        type: t,
        label: TYPE_LABELS[t] ?? t,
        total: thisYear.get(t) ?? 0,
      }))
    }

    if (group.limitFor) {
      const { base, catchUp } = group.limitFor(limits, age, hsaCoverage)
      row.limitBase = base
      row.catchUpAmount = catchUp
      row.limit = base + (catchUp ?? 0)
      row.pctUsed = row.limit > 0 ? total / row.limit : undefined
    }

    rows.push(row)
  }

  rows.sort((a, b) => a._order - b._order)
  return rows.map(({ _order, ...rest }) => rest)
}
