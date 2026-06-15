export const PAY_PERIODS = ['weekly', 'biweekly', 'semimonthly', 'monthly', 'irregular'] as const
export type PayPeriod = typeof PAY_PERIODS[number]

export const PAY_PERIOD_LABELS: Record<PayPeriod, string> = {
  weekly: 'Weekly',
  biweekly: 'Biweekly',
  semimonthly: 'Semi-monthly',
  monthly: 'Monthly',
  irregular: 'Irregular',
}

export const labelForPayPeriod = (period: string): string =>
  PAY_PERIOD_LABELS[period as PayPeriod] ?? period

export const payPeriodItems = PAY_PERIODS.map((p) => ({
  label: PAY_PERIOD_LABELS[p],
  value: p,
}))
