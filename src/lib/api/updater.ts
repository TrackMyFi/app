import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

/**
 * Thin wrappers around the Tauri updater/process plugins. The updater plugin
 * exposes a JS API directly (no Rust command needed); these keep the store free
 * of plugin imports and make the surface easy to mock in tests.
 */

/** Check the configured GitHub Releases endpoint. Returns null when up to date. */
export function checkForUpdate(): Promise<Update | null> {
  return check()
}

/** Download + install an update, reporting byte progress via the callback. */
export function downloadAndInstall(
  update: Update,
  onEvent: (event: DownloadEvent) => void,
): Promise<void> {
  return update.downloadAndInstall(onEvent)
}

/** Relaunch the app so the freshly installed version takes over. */
export function relaunchApp(): Promise<void> {
  return relaunch()
}

export type { Update, DownloadEvent }
