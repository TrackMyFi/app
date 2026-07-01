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
  savings: 'Contributions',
  fixed: 'Bills',
  discretionary: 'Spending',
  uncategorized: 'Uncategorized',
}

export const labelForCategory = (category: string): string =>
  CATEGORY_LABELS[category as Category] ?? category

export const categoryItems = CATEGORIES.map((c) => ({
  label: CATEGORY_LABELS[c],
  value: c,
}))

/**
 * Signed effect of a transaction on its PRIMARY account's balance.
 *
 * Balances are stored as positive magnitudes; the account TYPE carries the sign.
 * For a liability (debt owed), the effect of income/expense inverts: a purchase
 * (expense) raises what you owe, a refund (income) lowers it.
 *
 * Transfers default to the primary account's source perspective and decrease
 * its running balance — correct for an imported liability payment (the card
 * is the receiving side and its debt drops by `amount`) and for an asset
 * account that is the true source. Pass `direction: 'in'` for a non-liability
 * account that is actually the transfer's *destination* (an inbound
 * asset-to-asset transfer, e.g. importing the receiving side's CSV first) —
 * its balance rises by `amount` instead.
 */
export function signedDelta(
  type: string,
  amount: number,
  isLiability = false,
  direction: 'in' | 'out' = 'out',
): number {
  if (type === 'transfer') return !isLiability && direction === 'in' ? amount : -amount
  const sign = isLiability ? -1 : 1
  return type === 'income' ? sign * amount : sign * -amount
}

/**
 * Effect of one leg of a transfer on a positive-magnitude balance.
 * Assets: the source falls and the destination rises. A liability stores debt,
 * so the signs invert — sending money out of a card (source) raises its debt,
 * paying a card (destination) lowers it.
 */
export function transferLegDelta(
  leg: 'source' | 'destination',
  amount: number,
  isLiability = false,
): number {
  const assetDelta = leg === 'source' ? -amount : amount
  return isLiability ? -assetDelta : assetDelta
}
