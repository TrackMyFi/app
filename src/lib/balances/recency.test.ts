import { describe, it, expect } from 'vitest'
import { isNewer, latestSnapshot, byRecencyAsc, byRecencyDesc } from './recency'

describe('recency ordering', () => {
  it('treats a later date as newer regardless of id', () => {
    expect(isNewer({ recordedAt: '2026-02-02', id: 1 }, { recordedAt: '2026-02-01', id: 9 })).toBe(true)
  })

  it('breaks same-date ties by highest id', () => {
    expect(isNewer({ recordedAt: '2026-02-01', id: 2 }, { recordedAt: '2026-02-01', id: 1 })).toBe(true)
    expect(isNewer({ recordedAt: '2026-02-01', id: 1 }, { recordedAt: '2026-02-01', id: 2 })).toBe(false)
  })

  // The reported bug: a $500 expense recorded the same day as the $1800 snapshot
  // materializes a $1300 snapshot with a higher id. Account order from the DB is
  // ascending recorded_at only, so the stale $1800 row comes first.
  it('latestSnapshot picks the higher-id same-date snapshot, not array order', () => {
    const balances = [
      { id: 1, accountId: 10, balance: 1800, recordedAt: '2026-06-12' },
      { id: 2, accountId: 10, balance: 1300, recordedAt: '2026-06-12' },
    ]
    expect(latestSnapshot(balances)?.balance).toBe(1300)
  })

  it('byRecencyAsc sorts oldest first with same-date ties by id', () => {
    const balances = [
      { id: 2, balance: 1300, recordedAt: '2026-06-12' },
      { id: 1, balance: 1800, recordedAt: '2026-06-12' },
      { id: 3, balance: 2000, recordedAt: '2026-07-01' },
    ]
    expect([...balances].sort(byRecencyAsc).map(b => b.balance)).toEqual([1800, 1300, 2000])
  })

  it('byRecencyDesc sorts newest first with same-date ties newest by id', () => {
    const balances = [
      { id: 1, balance: 1800, recordedAt: '2026-06-12' },
      { id: 2, balance: 1300, recordedAt: '2026-06-12' },
      { id: 3, balance: 2000, recordedAt: '2026-07-01' },
    ]
    expect([...balances].sort(byRecencyDesc).map(b => b.balance)).toEqual([2000, 1300, 1800])
  })

  it('falls back to stable order when ids are absent', () => {
    const a = { recordedAt: '2026-06-12' }
    const b = { recordedAt: '2026-06-12' }
    expect(isNewer(a, b)).toBe(false)
    expect(byRecencyAsc(a, b)).toBe(0)
  })
})
