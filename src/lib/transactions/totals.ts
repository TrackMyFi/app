export interface AmountRow { type: string; amount: number }

export function runningTotals(rows: AmountRow[]): {
  income: number; expense: number; net: number
} {
  let income = 0
  let expense = 0
  for (const r of rows) {
    if (r.type === 'income') income += r.amount
    else if (r.type === 'expense') expense += r.amount
  }
  return { income, expense, net: income - expense }
}
