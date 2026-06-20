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
}

export interface ExistingRef {
  accountId: number
  date: string
  amount: number
  description: string
  type?: string
  transferAccountId?: number | null
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

    if (matchedTransferRule) {
      let amount: number
      if (config.amountMode === 'split') {
        const { amount: a } = resolveSplit(row, config)
        amount = a
      } else {
        amount = Math.abs(parseAmount(row[config.amountColumn] ?? '0'))
      }
      return {
        date,
        amount,
        description,
        type: 'transfer' as const,
        category: 'uncategorized',
        transferAccountId: matchedTransferRule.transferAccountId,
      }
    }

    if (config.amountMode === 'split') {
      const { amount, type } = resolveSplit(row, config)
      return { date, amount, description, type, category, transferAccountId: null }
    }

    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    return {
      date,
      amount: Math.abs(signed),
      description,
      type: isExpense ? 'expense' : 'income',
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
 * of an existing transfer whose *other* side is `accountId`.
 *
 * A transfer between two of your accounts is stored as a single row on one
 * account (with `transferAccountId` pointing at the other). When you later
 * import the other account's statement, the same event shows up with a
 * different description and often a slightly different date, so it can't be
 * matched on the exact date|amount|description key. Here we match on amount +
 * a date window instead, ignoring the description.
 */
export function detectTransferCounterparts(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
  toleranceDays = TRANSFER_DATE_TOLERANCE_DAYS,
): boolean[] {
  // Existing transfers whose other side is the account being imported into.
  // Their primary accountId is necessarily a different account.
  const counterparts = existing.filter(
    (e) => e.type === 'transfer' && e.transferAccountId === accountId,
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
