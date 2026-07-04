import { onMounted, onUnmounted } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useToast } from '@nuxt/ui/composables'
import { onSyncStatus } from '../lib/api/sync'
import { onSimpleFinSyncing } from '../lib/api/simplefin'
import type { SyncStatus } from '../lib/types/SyncStatus'

/**
 * Global toast notifications for background data-update jobs, so the user can
 * tell when their data is being touched: cloud sync (Turso) and bank sync
 * (SimpleFIN). Shows a persistent "working…" toast while a job runs, then
 * replaces it with a success/error outcome. Mounted once in App.vue.
 */
export function useSyncNotifications() {
  const toast = useToast()
  const unlisteners: UnlistenFn[] = []

  // In-flight toast ids, so the outcome replaces the progress toast instead
  // of piling up.
  let cloudToastId: string | number | undefined
  let bankToastId: string | number | undefined
  let prevCloudStatus: SyncStatus['status'] | undefined

  onMounted(async () => {
    unlisteners.push(
      await onSyncStatus((s) => {
        const prev = prevCloudStatus
        prevCloudStatus = s.status
        if (s.status === 'syncing') {
          if (cloudToastId == null) {
            cloudToastId = toast.add({
              title: 'Syncing with cloud…',
              description: 'Backing up your data to Turso.',
              icon: 'i-ph-cloud-arrow-up',
              color: 'info',
              duration: 60_000,
            }).id
          }
          return
        }
        // Only report an outcome for a transition we watched start — the
        // initial status snapshot on page load isn't an outcome.
        if (prev !== 'syncing') return
        if (cloudToastId != null) toast.remove(cloudToastId)
        cloudToastId = undefined
        if (s.status === 'error') {
          toast.add({
            title: 'Cloud sync failed',
            description: s.lastError ?? undefined,
            icon: 'i-ph-cloud-slash',
            color: 'error',
          })
        } else {
          toast.add({
            title: 'Cloud sync complete',
            icon: 'i-ph-cloud-check',
            color: 'success',
          })
        }
      }),
    )

    unlisteners.push(
      await onSimpleFinSyncing((e) => {
        if (e.syncing) {
          if (bankToastId == null) {
            bankToastId = toast.add({
              title: 'Updating bank data…',
              description: 'Pulling balances and transactions from SimpleFIN.',
              icon: 'i-ph-bank',
              color: 'info',
              duration: 120_000,
            }).id
          }
          return
        }
        if (bankToastId != null) toast.remove(bankToastId)
        bankToastId = undefined
        if (e.error) {
          toast.add({
            title: 'Bank sync failed',
            description: e.error,
            icon: 'i-ph-bank',
            color: 'error',
          })
        } else if (e.transactionsAdded > 0 || e.snapshotsAdded > 0) {
          const txns = `${e.transactionsAdded} ${e.transactionsAdded === 1 ? 'transaction' : 'transactions'}`
          const snaps = `${e.snapshotsAdded} balance ${e.snapshotsAdded === 1 ? 'update' : 'updates'}`
          toast.add({
            title: 'Bank data updated',
            description: `${txns}, ${snaps}.`,
            icon: 'i-ph-bank',
            color: 'success',
          })
        } else {
          toast.add({
            title: 'Bank data is up to date',
            icon: 'i-ph-bank',
            color: 'success',
          })
        }
      }),
    )
  })

  onUnmounted(() => unlisteners.forEach((u) => u()))
}
