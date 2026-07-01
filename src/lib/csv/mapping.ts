import { DateTime } from 'luxon'

export type AmountSign = 'negative-is-expense' | 'positive-is-expense'
export type AmountMode = 'single' | 'split'

export interface TransferRuleInput {
  keyword: string
  transferAccountId: number
}

export interface MappingConfig {
  dateColumn: string
  descriptionColumn: string
  dateFormat: string
  amountMode: AmountMode
  amountColumn: string
  amountSign: AmountSign
  creditColumn: string
  debitColumn: string
  invertSplit: boolean
  defaultCategory: string
  transferRules: TransferRuleInput[]
}

export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense' | 'transfer'
  category: string
  transferAccountId: number | null
  /**
   * Only meaningful when `type === 'transfer'`. 'out' means the raw row was
   * expense-shaped (money left the currently-imported account — it's the
   * true source). 'in' means it was income-shaped (money arrived — the
   * currently-imported account is really the destination, and the caller
   * should swap accountId/transferAccountId before storing). Left `undefined`
   * for non-transfer rows.
   */
  direction?: 'in' | 'out'
}

export interface ExistingRef {
  accountId: number
  date: string
  amount: number
  description: string
  type?: string
  transferAccountId?: number | null
  paycheckId?: number | null
}

export interface CategoryRuleInput {
  keyword: string
  category: string
}

export function autoDetectMapping(
  headers: string[],
  rows: Record<string, string>[],
): Partial<MappingConfig> {
  const find = (aliases: string[]): string =>
    headers.find((h) => {
      const normalized = h.toLowerCase().trim()
      return aliases.some((a) => normalized.includes(a))
    }) ?? ''

  const dateCol = find(['date'])
  const descCol = find(['description', 'memo', 'details', 'narrative', 'payee', 'merchant'])
  const amountCol = find(['amount', 'amt'])
  const creditCol = find(['credit', 'deposit'])
  const debitCol = find(['debit', 'withdrawal', 'charge'])

  const result: Partial<MappingConfig> = {}
  if (dateCol) result.dateColumn = dateCol
  if (descCol) result.descriptionColumn = descCol

  if (creditCol && debitCol) {
    result.amountMode = 'split'
    result.creditColumn = creditCol
    result.debitColumn = debitCol
  } else if (amountCol) {
    result.amountMode = 'single'
    result.amountColumn = amountCol
  }

  if (dateCol) {
    const sample = rows.find((r) => (r[dateCol] ?? '').trim())
    if (sample) {
      const raw = sample[dateCol].trim()
      const formats = ['MM/dd/yyyy', 'yyyy-MM-dd', 'M/d/yyyy', 'dd/MM/yyyy']
      const detected = formats.find((f) => DateTime.fromFormat(raw, f).isValid)
      if (detected) result.dateFormat = detected
    }
  }

  return result
}

export function parseAmount(raw: string): number {
  const s = (raw ?? '').trim()
  // Some bank exports encode negatives as (42.50) rather than -42.50
  const neg = s.startsWith('(') && s.endsWith(')')
  const n = Number((neg ? s.slice(1, -1) : s).replace(/[$,\s]/g, ''))
  return isNaN(n) ? 0 : neg ? -n : n
}

function isoDate(raw: string, format: string): string {
  return DateTime.fromFormat(raw ?? '', format).toISODate() ?? (raw ?? '')
}

function resolveSplit(
  row: Record<string, string>,
  config: MappingConfig,
): { amount: number; type: 'income' | 'expense' } {
  const credit = parseAmount(row[config.creditColumn])
  const debit = parseAmount(row[config.debitColumn])
  // credit = income, debit = expense; invertSplit flips the rule for banks that
  // export the columns the other way round. Account type does NOT enter here —
  // types stay intuitive (a card purchase is an expense). The liability sign is
  // applied later, when a balance delta is computed from the type.
  const creditIsIncome = !config.invertSplit

  if (credit === 0 && debit === 0) return { amount: 0, type: 'expense' }

  if (credit !== 0 && debit !== 0) {
    if (Math.abs(credit) >= Math.abs(debit)) {
      return { amount: Math.abs(credit), type: creditIsIncome ? 'income' : 'expense' }
    }
    return { amount: Math.abs(debit), type: creditIsIncome ? 'expense' : 'income' }
  }

  if (credit !== 0) return { amount: Math.abs(credit), type: creditIsIncome ? 'income' : 'expense' }
  return { amount: Math.abs(debit), type: creditIsIncome ? 'expense' : 'income' }
}

/** Transform raw CSV objects into parsed transactions using a mapping config. */
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
  rules: CategoryRuleInput[] = [],
): ParsedTransaction[] {
  const transferRules = config.transferRules ?? []

  return rows.map((row) => {
    const date = isoDate(row[config.dateColumn] ?? '', config.dateFormat)
    const description = row[config.descriptionColumn] ?? ''
    const descLower = description.toLowerCase()

    const matchedCategoryRule = rules.find((r) => descLower.includes(r.keyword.toLowerCase()))
    const category = matchedCategoryRule ? matchedCategoryRule.category : config.defaultCategory

    const matchedTransferRule = transferRules.find((r) => descLower.includes(r.keyword.toLowerCase()))

    // The row's natural (non-transfer) amount/direction, computed once and
    // reused both for plain income/expense rows and — when a transfer rule
    // matches — to infer which side of the transfer the currently-imported
    // account is really on (see `ParsedTransaction.direction`).
    const natural: { amount: number; type: 'income' | 'expense' } =
      config.amountMode === 'split'
        ? resolveSplit(row, config)
        : (() => {
            const signed = parseAmount(row[config.amountColumn] ?? '0')
            const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
            return { amount: Math.abs(signed), type: isExpense ? 'expense' : 'income' }
          })()

    if (matchedTransferRule) {
      return {
        date,
        amount: natural.amount,
        description,
        type: 'transfer' as const,
        category: 'uncategorized',
        transferAccountId: matchedTransferRule.transferAccountId,
        direction: natural.type === 'expense' ? ('out' as const) : ('in' as const),
      }
    }

    return {
      date,
      amount: natural.amount,
      description,
      type: natural.type,
      category,
      transferAccountId: null,
    }
  })
}

/** Return a parallel array: true where the parsed row duplicates an existing transaction. */
export function detectDuplicates(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
): boolean[] {
  const key = (date: string, amount: number, description: string) =>
    `${date}|${amount}|${description}`
  const seen = new Set(
    existing
      .filter((e) => e.accountId === accountId)
      .map((e) => key(e.date, e.amount, e.description)),
  )
  return parsed.map((p) => seen.has(key(p.date, p.amount, p.description)))
}

/** Default ± window (in days) for matching the two sides of a transfer. */
export const TRANSFER_DATE_TOLERANCE_DAYS = 3

/**
 * Return a parallel array: true where the parsed row looks like the counterpart
 * of an existing transfer touching `accountId` on either side.
 *
 * A transfer between two of your accounts is stored as a single canonical row
 * — source account as `accountId`, destination as `transferAccountId` — but
 * which of your two accounts ends up as which depends only on which one was
 * imported first (see `direction` on `ParsedTransaction`), not on import
 * order. So when you later import the *other* account's statement, the same
 * event shows up with a different description and often a slightly different
 * date, and the existing canonical row may have `accountId` on either side of
 * it. It can't be matched on the exact date|amount|description key, so we
 * match on amount + a date window instead, ignoring the description. This is
 * a best-effort heuristic: two unrelated transfers of the same amount within
 * the window would also match.
 */
export function detectTransferCounterparts(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
  toleranceDays = TRANSFER_DATE_TOLERANCE_DAYS,
): boolean[] {
  // Existing transfers touching this account, on either side.
  const counterparts = existing.filter(
    (e) => e.type === 'transfer' && (e.transferAccountId === accountId || e.accountId === accountId),
  )
  const consumed = new Array(counterparts.length).fill(false)

  return parsed.map((p) => {
    const pDate = DateTime.fromISO(p.date)
    const matchIdx = counterparts.findIndex(
      (c, i) =>
        !consumed[i] &&
        Math.abs(c.amount - p.amount) < 0.005 &&
        Math.abs(DateTime.fromISO(c.date).diff(pDate, 'days').days) <= toleranceDays,
    )
    if (matchIdx === -1) return false
    consumed[matchIdx] = true
    return true
  })
}

/**
 * Return a parallel array: true where a parsed income row looks like the
 * deposit for an existing paycheck-generated transaction on this account.
 *
 * Paycheck income txns carry a synthetic description ("Paycheck – Employer")
 * that never matches the bank's own description text, so `detectDuplicates`
 * can't catch them. Match on amount + a date window instead, same approach
 * as `detectTransferCounterparts`.
 */
export function detectPaycheckDuplicates(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
  toleranceDays = TRANSFER_DATE_TOLERANCE_DAYS,
): boolean[] {
  const candidates = existing.filter(
    (e) => e.accountId === accountId && e.paycheckId != null,
  )
  const consumed = new Array(candidates.length).fill(false)

  return parsed.map((p) => {
    if (p.type !== 'income') return false
    const pDate = DateTime.fromISO(p.date)
    const matchIdx = candidates.findIndex(
      (c, i) =>
        !consumed[i] &&
        Math.abs(c.amount - p.amount) < 0.005 &&
        Math.abs(DateTime.fromISO(c.date).diff(pDate, 'days').days) <= toleranceDays,
    )
    if (matchIdx === -1) return false
    consumed[matchIdx] = true
    return true
  })
}
