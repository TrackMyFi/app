import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { SimpleFinDuplicateCandidate } from '../types/SimpleFinDuplicateCandidate'
import type { SimpleFinPendingTransaction } from '../types/SimpleFinPendingTransaction'
import type { SimpleFinStatus } from '../types/SimpleFinStatus'
import type { SimpleFinSyncSummary } from '../types/SimpleFinSyncSummary'
import type { SimpleFinSyncingEvent } from '../types/SimpleFinSyncingEvent'

export const getSimpleFinStatus = () => invoke<SimpleFinStatus>('simplefin_get_status')

/** Claims the one-time setup token and runs a first sync. */
export const connectSimpleFin = (setupToken: string) =>
  invoke<SimpleFinStatus>('simplefin_connect', { setupToken })

/** Link a SimpleFIN account to a local account; pass null to unlink. */
export const linkSimpleFinAccount = (simplefinId: string, accountId: number | null) =>
  invoke<void>('simplefin_link_account', { simplefinId, accountId })

export const syncSimpleFinNow = () => invoke<SimpleFinSyncSummary>('simplefin_sync_now')

/** Backfill a specific date range (inclusive, "yyyy-MM-dd"). Imports posted
 *  transactions only; dedup makes overlap with existing history harmless. */
export const syncSimpleFinRange = (startDate: string, endDate: string) =>
  invoke<SimpleFinSyncSummary>('simplefin_sync_range', { startDate, endDate })

/** Candidate SimpleFIN-vs-manual/CSV duplicate pairs for the review UI. */
export const listSimpleFinDuplicates = () =>
  invoke<SimpleFinDuplicateCandidate[]>('simplefin_duplicate_candidates')

/** Transactions still pending at the bank — awareness only, outside the
 *  ledger and every sum/aggregate. Empty when SimpleFIN isn't connected. */
export const listSimpleFinPending = () =>
  invoke<SimpleFinPendingTransaction[]>('simplefin_list_pending')

export const disconnectSimpleFin = () => invoke<void>('simplefin_disconnect')

export const onSimpleFinStatus = (cb: (s: SimpleFinStatus) => void): Promise<UnlistenFn> =>
  listen<SimpleFinStatus>('simplefin-status', (e) => cb(e.payload))

/** Fires when a SimpleFIN sync starts (`syncing: true`) and finishes (`syncing: false` + outcome). */
export const onSimpleFinSyncing = (cb: (e: SimpleFinSyncingEvent) => void): Promise<UnlistenFn> =>
  listen<SimpleFinSyncingEvent>('simplefin-syncing', (e) => cb(e.payload))
