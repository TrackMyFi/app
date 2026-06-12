export const TRANSACTION_TYPES = ['income', 'expense', 'transfer'] as const
export type TransactionType = typeof TRANSACTION_TYPES[number]

export const CATEGORIES = ['savings', 'fixed', 'discretionary', 'uncategorized'] as const
export type Category = typeof CATEGORIES[number]

/** Signed effect of a transaction on its PRIMARY account's balance. */
export function signedDelta(type: string, amount: number): number {
  if (type === 'income') return amount
  if (type === 'expense') return -amount
  return -amount // transfer: primary (source) account decreases
}
