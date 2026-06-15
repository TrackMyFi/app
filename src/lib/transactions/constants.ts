export const TRANSACTION_TYPES = ['income', 'expense', 'transfer'] as const
export type TransactionType = typeof TRANSACTION_TYPES[number]

export const TRANSACTION_TYPE_LABELS: Record<TransactionType, string> = {
  income: 'Income',
  expense: 'Expense',
  transfer: 'Transfer',
}

export const labelForTransactionType = (type: string): string =>
  TRANSACTION_TYPE_LABELS[type as TransactionType] ?? type

export const transactionTypeItems = TRANSACTION_TYPES.map((t) => ({
  label: TRANSACTION_TYPE_LABELS[t],
  value: t,
}))

export const CATEGORIES = ['savings', 'fixed', 'discretionary', 'uncategorized'] as const
export type Category = typeof CATEGORIES[number]

export const CATEGORY_LABELS: Record<Category, string> = {
  savings: 'Savings',
  fixed: 'Fixed',
  discretionary: 'Discretionary',
  uncategorized: 'Uncategorized',
}

export const labelForCategory = (category: string): string =>
  CATEGORY_LABELS[category as Category] ?? category

export const categoryItems = CATEGORIES.map((c) => ({
  label: CATEGORY_LABELS[c],
  value: c,
}))

/** Signed effect of a transaction on its PRIMARY account's balance. */
export function signedDelta(type: string, amount: number): number {
  if (type === 'income') return amount
  if (type === 'expense') return -amount
  return -amount // transfer: primary (source) account decreases
}
