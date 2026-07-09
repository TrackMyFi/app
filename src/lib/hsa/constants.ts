export const HSA_EXPENSE_CATEGORIES = ['medical', 'dental', 'vision', 'pharmacy', 'other'] as const
export type HsaExpenseCategory = typeof HSA_EXPENSE_CATEGORIES[number]

export const HSA_EXPENSE_CATEGORY_LABELS: Record<HsaExpenseCategory, string> = {
  medical: 'Medical',
  dental: 'Dental',
  vision: 'Vision',
  pharmacy: 'Pharmacy',
  other: 'Other',
}

export const labelForHsaCategory = (category: string): string =>
  HSA_EXPENSE_CATEGORY_LABELS[category as HsaExpenseCategory] ?? category

export const hsaCategoryItems = HSA_EXPENSE_CATEGORIES.map((c) => ({
  label: HSA_EXPENSE_CATEGORY_LABELS[c],
  value: c,
}))

// NuxtUI semantic color per category, used for badges.
export const HSA_EXPENSE_CATEGORY_COLOR: Record<HsaExpenseCategory, 'primary' | 'success' | 'info' | 'warning' | 'neutral'> = {
  medical: 'primary',
  dental: 'info',
  vision: 'success',
  pharmacy: 'warning',
  other: 'neutral',
}

export const colorForHsaCategory = (category: string) =>
  HSA_EXPENSE_CATEGORY_COLOR[category as HsaExpenseCategory] ?? 'neutral'
