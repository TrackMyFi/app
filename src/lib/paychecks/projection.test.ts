import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { projectYearEnd } from './projection'
import type { Paycheck } from '../types/Paycheck'

let nextId = 1

function paycheck(overrides: Partial<Paycheck>): Paycheck {
  return {
    id: nextId++,
    payDate: '2026-01-15',
    employer: 'Acme',
    payPeriod: 'biweekly',
    grossAmount: 4000,
    netAmount: 3000,
    federalTax: 600,
    stateTax: 150,
    localTax: 0,
    socialSecurityTax: 248,
    medicareTax: 58,
    deductions: [],
    employerMatch: [],
    incomeAccountId: null,
    importSource: 'manual',
    createdAt: '2026-01-15T00:00:00Z',
    updatedAt: '2026-01-15T00:00:00Z',
    ...overrides,
  }
}

// Biweekly paychecks landing every 14 days from Jan 9 through `throughIso`.
function biweeklySeries(throughIso: string, overrides: Partial<Paycheck> = {}): Paycheck[] {
  const out: Paycheck[] = []
  let d = DateTime.fromISO('2026-01-09')
  const through = DateTime.fromISO(throughIso)
  while (d <= through) {
    out.push(paycheck({ payDate: d.toISODate()!, ...overrides }))
    d = d.plus({ days: 14 })
  }
  return out
}

describe('projectYearEnd', () => {
  const today = DateTime.fromISO('2026-07-05')

  it('returns null when there are no paychecks', () => {
    expect(projectYearEnd([], today)).toBeNull()
  })

  it('returns null when every paycheck is irregular', () => {
    const rows = [
      paycheck({ payPeriod: 'irregular', payDate: '2026-03-15' }),
      paycheck({ payPeriod: 'irregular', payDate: '2026-06-01' }),
    ]
    expect(projectYearEnd(rows, today)).toBeNull()
  })

  it('extends a biweekly stream to Dec 31 at its average paycheck', () => {
    const rows = biweeklySeries('2026-07-03') // 13 paychecks, Jan 9 – Jun 26
    const result = projectYearEnd(rows, today)!
    // Jun 26 → Dec 31 is 188 days = 13 more biweekly paychecks (26 for the year).
    expect(result.remainingCount).toBe(13)
    expect(result.projectedGross).toBe(26 * 4000)
    expect(result.projectedNet).toBe(26 * 3000)
  })

  it('includes irregular actuals in the projection without extrapolating them', () => {
    const rows = [
      ...biweeklySeries('2026-07-03'),
      paycheck({ payPeriod: 'irregular', payDate: '2026-03-15', grossAmount: 10000, netAmount: 6500 }),
    ]
    const result = projectYearEnd(rows, today)!
    expect(result.remainingCount).toBe(13) // the bonus adds no future paychecks
    expect(result.projectedGross).toBe(26 * 4000 + 10000)
    expect(result.projectedNet).toBe(26 * 3000 + 6500)
  })

  it('drops a stream that went quiet (job ended mid-year)', () => {
    const rows = [
      ...biweeklySeries('2026-07-03'),
      // Old job's stream stopped in March — well past the grace window.
      ...biweeklySeries('2026-03-27', { employer: 'OldCorp', grossAmount: 2000, netAmount: 1500 }),
    ]
    const result = projectYearEnd(rows, today)!
    expect(result.remainingCount).toBe(13) // only Acme projects forward
    const oldCorpActuals = biweeklySeries('2026-03-27').length
    expect(result.projectedGross).toBe(26 * 4000 + oldCorpActuals * 2000)
  })

  it('projects separate streams for two active employers', () => {
    const rows = [
      ...biweeklySeries('2026-07-03'),
      paycheck({ employer: 'SideGig', payPeriod: 'monthly', payDate: '2026-06-30', grossAmount: 1000, netAmount: 800 }),
    ]
    const result = projectYearEnd(rows, today)!
    // Jun 30 → Dec 31 is 184 days ≈ 6 monthly paychecks on top of 13 biweekly.
    expect(result.remainingCount).toBe(19)
    expect(result.projectedGross).toBe(26 * 4000 + 7 * 1000)
    expect(result.projectedNet).toBe(26 * 3000 + 7 * 800)
  })

  it('survives an entry lag within the grace window', () => {
    // Last recorded paycheck was ~3 weeks ago — one missed biweekly cycle,
    // inside the two-interval grace. Still projects.
    const rows = biweeklySeries('2026-06-12')
    const result = projectYearEnd(rows, today)!
    expect(result.remainingCount).toBeGreaterThan(0)
  })

  it('returns actuals with zero remaining when the year is fully paid out', () => {
    const rows = biweeklySeries('2026-12-25')
    const dec31 = DateTime.fromISO('2026-12-31')
    const result = projectYearEnd(rows, dec31)!
    expect(result.remainingCount).toBe(0)
    expect(result.projectedGross).toBe(rows.length * 4000)
  })

  it('averages an uneven stream rather than trusting the last paycheck', () => {
    const rows = [
      paycheck({ payDate: '2026-06-05', grossAmount: 3000, netAmount: 2200 }),
      paycheck({ payDate: '2026-06-19', grossAmount: 5000, netAmount: 3800 }), // overtime spike
    ]
    const result = projectYearEnd(rows, today)!
    // Jun 19 → Dec 31 is 195 days = 13 more at the $4,000 average.
    expect(result.remainingCount).toBe(13)
    expect(result.projectedGross).toBe(8000 + 13 * 4000)
    expect(result.projectedNet).toBe(6000 + 13 * 3000)
  })
})
