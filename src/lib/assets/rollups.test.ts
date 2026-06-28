import { describe, it, expect } from 'vitest'
import type { AssetEvent } from '../types/AssetEvent'
import {
  lifetimeCost,
  costBasisAdded,
  purchaseCost,
  upkeepCost,
  yearsSpanned,
  annualizedCost,
  currentValue,
  groupByAsset,
} from './rollups'

const ev = (over: Partial<AssetEvent>): AssetEvent => ({
  id: 1,
  accountId: null,
  assetLabel: null,
  date: '2026-01-01',
  description: 'event',
  kind: 'maintenance',
  cost: 100,
  assetValue: null,
  vendor: null,
  notes: null,
  lifeExpectancy: null,
  linkedTransactionId: null,
  createdAt: '2026-01-01',
  updatedAt: '2026-01-01',
  hasAttachment: false,
  ...over,
})

const events: AssetEvent[] = [
  ev({ id: 1, accountId: 5, kind: 'improvement', cost: 12000, date: '2020-06-01' }),
  ev({ id: 2, accountId: 5, kind: 'maintenance', cost: 300, date: '2024-06-01' }),
  ev({ id: 3, assetLabel: '2019 Honda CR-V', kind: 'repair', cost: 150, date: '2026-06-01' }),
]

describe('asset rollups', () => {
  it('lifetimeCost sums all events', () => {
    expect(lifetimeCost(events)).toBe(12450)
  })

  it('costBasisAdded sums only improvements', () => {
    expect(costBasisAdded(events)).toBe(12000)
  })

  it('upkeepCost sums maintenance + repair, excluding improvements', () => {
    expect(upkeepCost(events)).toBe(450)
  })

  it('yearsSpanned measures earliest to latest, floored at 1', () => {
    expect(yearsSpanned(events)).toBeCloseTo(6.0, 1)
    expect(yearsSpanned([events[0]])).toBe(1)
    expect(yearsSpanned([])).toBe(1)
    // sub-year span floors to 1
    expect(
      yearsSpanned([ev({ date: '2026-01-01' }), ev({ date: '2026-03-01' })]),
    ).toBe(1)
  })

  it('annualizedCost spreads lifetime cost across the span', () => {
    expect(annualizedCost(events)).toBeCloseTo(12450 / yearsSpanned(events), 5)
    expect(annualizedCost([])).toBe(0)
  })

  it('handles empty input across all rollups', () => {
    expect(lifetimeCost([])).toBe(0)
    expect(costBasisAdded([])).toBe(0)
    expect(upkeepCost([])).toBe(0)
  })

  it('groupByAsset keys by account id or label, sorted by lifetime cost', () => {
    const groups = groupByAsset(events)
    expect(groups).toHaveLength(2)
    // account 5 has 12300 total, the car has 150 -> account first
    expect(groups[0].accountId).toBe(5)
    expect(groups[0].events).toHaveLength(2)
    expect(groups[1].label).toBe('2019 Honda CR-V')
    expect(groups[1].accountId).toBeNull()
  })

  it('treats a purchase as acquisition: counted in total/purchase, excluded from upkeep & reserve', () => {
    const car: AssetEvent[] = [
      ev({ id: 1, kind: 'purchase', cost: 30000, date: '2020-06-01', assetValue: 30000 }),
      ev({ id: 2, kind: 'maintenance', cost: 300, date: '2022-06-01' }),
      ev({ id: 3, kind: 'repair', cost: 700, date: '2024-06-01' }),
    ]
    expect(purchaseCost(car)).toBe(30000)
    expect(lifetimeCost(car)).toBe(31000) // purchase IS real money spent
    expect(upkeepCost(car)).toBe(1000) // purchase excluded from upkeep
    expect(costBasisAdded(car)).toBe(0) // a purchase is not an improvement
    // reserve = ongoing (1000) spread over the full 4-year ownership span, not /1
    expect(yearsSpanned(car)).toBeCloseTo(4.0, 1)
    expect(annualizedCost(car)).toBeCloseTo(1000 / yearsSpanned(car), 5)
  })

  it('currentValue returns the most recent event value, or null when none set', () => {
    expect(currentValue([])).toBeNull()
    expect(currentValue([ev({ id: 1, date: '2024-01-01' })])).toBeNull()
    expect(
      currentValue([
        ev({ id: 1, date: '2024-01-01', assetValue: 20000 }),
        ev({ id: 2, date: '2026-01-01', assetValue: 15000 }),
        ev({ id: 3, date: '2025-01-01', assetValue: 17000 }),
      ]),
    ).toBe(15000)
    // a later, value-less event does not clobber the last known value
    expect(
      currentValue([
        ev({ id: 1, date: '2024-01-01', assetValue: 20000 }),
        ev({ id: 2, date: '2026-06-01', assetValue: null }),
      ]),
    ).toBe(20000)
    // tie on date -> higher id wins
    expect(
      currentValue([
        ev({ id: 5, date: '2026-01-01', assetValue: 9000 }),
        ev({ id: 9, date: '2026-01-01', assetValue: 8000 }),
      ]),
    ).toBe(8000)
  })

  it('groups label events case-insensitively', () => {
    const groups = groupByAsset([
      ev({ id: 10, assetLabel: 'Honda CR-V', cost: 100 }),
      ev({ id: 11, assetLabel: 'honda cr-v', cost: 50 }),
    ])
    expect(groups).toHaveLength(1)
    expect(groups[0].events).toHaveLength(2)
  })
})
