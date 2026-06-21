import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import {
  checkForUpdate,
  downloadAndInstall,
  relaunchApp,
  type Update,
} from '../lib/api/updater'

/** How often to re-check for updates while the app stays open (1 hour). */
export const CHECK_INTERVAL_MS = 1 * 60 * 60 * 1000

export type UpdateStatus =
  | 'idle' // no update known
  | 'checking' // querying the release endpoint
  | 'available' // an update exists, not yet downloading
  | 'downloading' // download in progress
  | 'ready' // downloaded + installed, waiting for relaunch
  | 'error' // last operation failed

export const useUpdaterStore = defineStore('updater', () => {
  const status = ref<UpdateStatus>('idle')
  const currentVersion = ref('')
  const newVersion = ref<string | null>(null)
  const notes = ref<string | null>(null)
  const error = ref<string | null>(null)
  /** 0–100 download progress, or null when not downloading. */
  const progress = ref<number | null>(null)
  /** True once the user dismisses the popover for this particular version. */
  const dismissed = ref(false)
  /** Epoch ms of the last completed check attempt (null until first check). */
  const lastCheckedAt = ref<number | null>(null)

  let pending: Update | null = null
  let timer: ReturnType<typeof setInterval> | null = null
  let contentLength = 0
  let downloaded = 0

  async function loadVersion() {
    if (!currentVersion.value) {
      try {
        currentVersion.value = await getVersion()
      } catch {
        // Running outside Tauri (plain `vite`); leave version blank.
      }
    }
  }

  /**
   * Check the release endpoint. `silent` suppresses the error state for the
   * automatic background checks so a flaky network never nags the user.
   */
  async function check(silent = false) {
    if (status.value === 'downloading' || status.value === 'ready') return
    await loadVersion()
    // A manual check always resurfaces the card (incl. the error "Try again").
    if (!silent) dismissed.value = false
    status.value = 'checking'
    lastCheckedAt.value = Date.now()
    error.value = null
    try {
      const update = await checkForUpdate()
      if (update) {
        // Background re-checks only re-nag when a newer version appears, so a
        // dismissed "Later" stays dismissed until something actually changes.
        if (update.version !== newVersion.value) dismissed.value = false
        pending = update
        newVersion.value = update.version
        notes.value = update.body ?? null
        status.value = 'available'
      } else {
        pending = null
        newVersion.value = null
        status.value = 'idle'
      }
    } catch (e) {
      if (!silent) {
        error.value = e instanceof Error ? e.message : String(e)
        status.value = 'error'
      } else {
        status.value = 'idle'
      }
    }
  }

  async function install() {
    if (!pending) return
    status.value = 'downloading'
    progress.value = 0
    contentLength = 0
    downloaded = 0
    error.value = null
    try {
      await downloadAndInstall(pending, (event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0
            break
          case 'Progress':
            downloaded += event.data.chunkLength
            progress.value = contentLength
              ? Math.min(100, Math.round((downloaded / contentLength) * 100))
              : null
            break
          case 'Finished':
            progress.value = 100
            break
        }
      })
      status.value = 'ready'
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      status.value = 'error'
    }
  }

  async function restart() {
    await relaunchApp()
  }

  function dismiss() {
    dismissed.value = true
  }

  /** Check on startup, then poll periodically. Safe to call once on app mount. */
  function init() {
    void check(true)
    if (!timer) {
      timer = setInterval(() => void check(true), CHECK_INTERVAL_MS)
    }
  }

  return {
    status,
    currentVersion,
    newVersion,
    notes,
    error,
    progress,
    dismissed,
    lastCheckedAt,
    check,
    install,
    restart,
    dismiss,
    init,
    loadVersion,
  }
})
