import { DateTime } from 'luxon'

export type AmountSign = 'negative-is-expense' | 'positive-is-expense'

export interface MappingConfig {
  dateColumn: string
  amountColumn: string
  descriptionColumn: string
  dateFormat: string
  amountSign: AmountSign
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

function parseAmount(raw: string): number {
  return Number(raw.replace(/[$,\s]/g, ''))
}

/** Transform raw CSV objects into parsed transactions using a mapping config. */
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
): ParsedTransaction[] {
  return rows.map((row) => {
    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense =
      config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    const iso =
      DateTime.fromFormat(row[config.dateColumn] ?? '', config.dateFormat).toISODate() ??
      (row[config.dateColumn] ?? '')
    return {
      date: iso,
      amount: Math.abs(signed),
      description: row[config.descriptionColumn] ?? '',
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
