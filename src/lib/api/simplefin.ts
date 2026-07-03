import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { SimpleFinStatus } from '../types/SimpleFinStatus'
import type { SimpleFinSyncSummary } from '../types/SimpleFinSyncSummary'

export const getSimpleFinStatus = () => invoke<SimpleFinStatus>('simplefin_get_status')

/** Claims the one-time setup token and runs a first sync. */
export const connectSimpleFin = (setupToken: string) =>
  invoke<SimpleFinStatus>('simplefin_connect', { setupToken })

/** Link a SimpleFIN account to a local account; pass null to unlink. */
export const linkSimpleFinAccount = (simplefinId: string, accountId: number | null) =>
  invoke<void>('simplefin_link_account', { simplefinId, accountId })

export const syncSimpleFinNow = () => invoke<SimpleFinSyncSummary>('simplefin_sync_now')

export const disconnectSimpleFin = () => invoke<void>('simplefin_disconnect')

export const onSimpleFinStatus = (cb: (s: SimpleFinStatus) => void): Promise<UnlistenFn> =>
  listen<SimpleFinStatus>('simplefin-status', (e) => cb(e.payload))
