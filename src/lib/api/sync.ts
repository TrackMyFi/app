import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { SyncStatus } from '../types/SyncStatus'

export const getSyncStatus = () => invoke<SyncStatus>('get_sync_status')

export const saveSyncConfig = (url: string, token: string) =>
  invoke<string>('save_sync_config', { url, token })

export const clearSyncConfig = () => invoke<void>('clear_sync_config')

export const syncNow = () => invoke<void>('sync_now')

export const restartApp = () => invoke<void>('restart_app')

// Tell the backend the `data-refreshed` listener is attached, so the post-startup
// catch-up refresh can be emitted without racing the listener registration.
export const frontendReady = () => invoke<void>('frontend_ready')

export const onSyncStatus = (cb: (s: SyncStatus) => void): Promise<UnlistenFn> =>
  listen<SyncStatus>('sync-status', (e) => cb(e.payload))
