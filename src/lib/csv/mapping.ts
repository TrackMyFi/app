import { DateTime } from 'luxon'

export type AmountSign = 'negative-is-expense' | 'positive-is-expense'
export type AmountMode = 'single' | 'split'

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
}

export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense'
  category: string
}

export interface ExistingRef {
  accountId: number
  date: string
  amount: number
  description: string
}

export function parseAmount(raw: string): number {
  return Number((raw ?? '').replace(/[$,\s]/g, ''))
}

function isoDate(raw: string, format: string): string {
  return DateTime.fromFormat(raw ?? '', format).toISODate() ?? (raw ?? '')
}

function resolveSplit(
  row: Record<string, string>,
  config: MappingConfig,
  isLiabilityAccount: boolean,
): { amount: number; type: 'income' | 'expense' } {
  const credit = parseAmount(row[config.creditColumn])
  const debit = parseAmount(row[config.debitColumn])
  // For a non-liability account: credit = income, debit = expense.
  // For a liability account: credit = expense, debit = income.
  // invertSplit flips the base rule.
  const creditIsIncome = !isLiabilityAccount !== config.invertSplit

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
  isLiabilityAccount = false,
): ParsedTransaction[] {
  return rows.map((row) => {
    const date = isoDate(row[config.dateColumn] ?? '', config.dateFormat)
    const description = row[config.descriptionColumn] ?? ''

    if (config.amountMode === 'split') {
      const { amount, type } = resolveSplit(row, config, isLiabilityAccount)
      return { date, amount, description, type, category: config.defaultCategory }
    }

    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    return {
      date,
      amount: Math.abs(signed),
      description,
      type: isExpense ? 'expense' : 'income',
      category: config.defaultCategory,
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
