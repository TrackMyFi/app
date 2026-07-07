<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useToast } from '@nuxt/ui/composables'
import { useSimpleFinStore } from '../../stores/simplefin'
import { useAccountsStore } from '../../stores/accounts'
import { createAccount } from '../../lib/api/accounts'
import type { SimpleFinRemoteAccount } from '../../lib/types/SimpleFinRemoteAccount'
import type { SimpleFinSyncSummary } from '../../lib/types/SimpleFinSyncSummary'
import DateInput from '../../components/DateInput.vue'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import SimpleFinDuplicateReview from '../../components/SimpleFinDuplicateReview.vue'
import { usePageData } from '../../composables/usePageData'

const toast = useToast()
const { error, run, retry } = usePageData()

const simplefin = useSimpleFinStore()
const accountsStore = useAccountsStore()

const setupToken = ref('')
const busy = ref(false)
const message = ref('')
const linkBusy = ref<string | null>(null)
const reviewOpen = ref(false)

onMounted(() =>
  run(async () => {
    await Promise.all([simplefin.load(), accountsStore.loadList()])
  }),
)

const status = computed(() => simplefin.status)
const isConnected = computed(() => status.value?.connected ?? false)

const lastSynced = computed(() => {
  const at = status.value?.lastSuccessAt
  return at ? new Date(at).toLocaleString() : 'never'
})

const money = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })

const NOT_LINKED = 'none'
const CREATE_NEW = 'create'

const linkItems = computed(() => [
  { label: 'Not linked', value: NOT_LINKED },
  ...accountsStore.accounts
    .filter((a) => a.isActive)
    .map((a) => ({ label: a.name, value: String(a.id) })),
  { label: '+ Create new account…', value: CREATE_NEW },
])

function linkValue(remote: SimpleFinRemoteAccount): string {
  return remote.linkedAccountId != null ? String(remote.linkedAccountId) : NOT_LINKED
}

const hasUnlinked = computed(
  () => (status.value?.accounts ?? []).some((a) => a.linkedAccountId == null),
)
const linkedCount = computed(
  () => (status.value?.accounts ?? []).filter((a) => a.linkedAccountId != null).length,
)

async function onLinkChange(remote: SimpleFinRemoteAccount, value: string) {
  linkBusy.value = remote.id
  message.value = ''
  try {
    if (value === CREATE_NEW) {
      // Negative bank balances are almost always debt — start those as a
      // liability; everything else starts as checking. Editable afterwards.
      const id = await createAccount({
        name: remote.name,
        type: remote.balance < 0 ? 'liability' : 'checking',
        institution: remote.org ?? null,
        includeInFireCalculations: false,
        countPaymentsAsExpense: false,
        createdAt: new Date().toISOString().slice(0, 10),
        traditionalPct: null,
      })
      await simplefin.link(remote.id, id)
      await accountsStore.loadList()
      toast.add({ title: `Created "${remote.name}" and linked it`, color: 'success' })
    } else if (value === NOT_LINKED) {
      await simplefin.link(remote.id, null)
    } else {
      await simplefin.link(remote.id, Number(value))
    }
  } catch (e) {
    message.value = String(e)
  } finally {
    linkBusy.value = null
  }
}

async function connect() {
  if (!setupToken.value.trim()) {
    message.value = 'Paste your SimpleFIN setup token first.'
    return
  }
  busy.value = true
  message.value = ''
  try {
    await simplefin.connect(setupToken.value)
    setupToken.value = ''
    toast.add({ title: 'SimpleFIN connected', color: 'success' })
  } catch (e) {
    message.value = String(e)
  } finally {
    busy.value = false
  }
}

function toastSummary(s: SimpleFinSyncSummary) {
  const transfers = s.transfersDetected > 0
    ? `, ${s.transfersDetected} ${s.transfersDetected === 1 ? 'transfer' : 'transfers'} detected`
    : ''
  toast.add({
    title: `Synced ${s.accountsSynced} ${s.accountsSynced === 1 ? 'account' : 'accounts'}: ${s.transactionsAdded} new ${s.transactionsAdded === 1 ? 'transaction' : 'transactions'}, ${s.snapshotsAdded} balance ${s.snapshotsAdded === 1 ? 'snapshot' : 'snapshots'}${transfers}`,
    color: 'success',
  })
}

async function syncNow() {
  busy.value = true
  message.value = ''
  try {
    toastSummary(await simplefin.syncNow())
    await accountsStore.loadList()
  } catch (e) {
    message.value = String(e)
  } finally {
    busy.value = false
  }
}

// --- custom-range backfill ---

/** Mirrors CUSTOM_RANGE_MAX_DAYS in src-tauri/src/simplefin.rs. */
const RANGE_MAX_DAYS = 365

const rangeOpen = ref(false)
const rangeStart = ref('')
const rangeEnd = ref('')

const syncMenuItems = [
  {
    label: 'Sync custom range…',
    icon: 'i-ph-calendar-blank',
    onSelect: openRangeModal,
  },
]

function openRangeModal() {
  const today = DateTime.now()
  rangeStart.value = today.minus({ days: 14 }).toISODate()
  rangeEnd.value = today.toISODate()
  rangeOpen.value = true
}

/** Same rules the backend enforces, surfaced before the request is made. */
const rangeError = computed(() => {
  if (!rangeStart.value || !rangeEnd.value) return 'Pick both dates.'
  const start = DateTime.fromISO(rangeStart.value)
  const end = DateTime.fromISO(rangeEnd.value)
  const today = DateTime.now().startOf('day')
  if (start > end) return 'The start date must be on or before the end date.'
  if (end > today) return "The end date can't be in the future."
  if (today.diff(start, 'days').days > RANGE_MAX_DAYS)
    return `SimpleFIN provides about a year of history — pick a start date within the last ${RANGE_MAX_DAYS} days.`
  return null
})

async function syncRange() {
  if (rangeError.value) return
  busy.value = true
  message.value = ''
  try {
    toastSummary(await simplefin.syncRange(rangeStart.value, rangeEnd.value))
    await accountsStore.loadList()
    rangeOpen.value = false
  } catch (e) {
    message.value = String(e)
    rangeOpen.value = false
  } finally {
    busy.value = false
  }
}

async function disconnect() {
  const ok = await confirm(
    'Disconnect SimpleFIN? Already-imported transactions and balances are kept. ' +
      'Reconnecting later requires a new setup token.',
    { title: 'Disconnect SimpleFIN', kind: 'warning' },
  )
  if (!ok) return
  busy.value = true
  try {
    await simplefin.disconnect()
    toast.add({ title: 'SimpleFIN disconnected', color: 'success' })
  } catch (e) {
    message.value = String(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="p-6 max-w-7xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <div v-else class="space-y-8">
      <section class="space-y-3">
        <h2 class="text-xl font-bold">Bank Sync (SimpleFIN)</h2>
        <p class="text-sm text-muted max-w-3xl">
          Optional. Pulls account balances and transactions from your banks once a day
          through <span class="font-mono">SimpleFIN Bridge</span>. The access credential is
          stored in your OS keychain; imported data is flagged with its source so it's easy
          to tell apart from manual entries.
        </p>

        <!-- Not connected -->
        <template v-if="!isConnected">
          <UFormField
            label="Setup token"
            hint="One-time token from your SimpleFIN Bridge account"
          >
            <UTextarea
              v-model="setupToken"
              :rows="3"
              placeholder="Paste the setup token here"
              class="w-full font-mono text-xs"
            />
          </UFormField>
          <UButton :loading="busy" @click="connect">Connect</UButton>

          <UAccordion
            :items="[{ label: 'How to set this up', slot: 'help', icon: 'i-ph-question' }]"
          >
            <template #help>
              <div class="text-sm space-y-2 p-2">
                <ol class="list-decimal ml-5 space-y-1">
                  <li>Go to <span class="font-mono">bridge.simplefin.org</span> and create an account (small yearly fee).</li>
                  <li>Connect your banks under <strong>Connections</strong>.</li>
                  <li>Open <strong>My Account → New App Connection</strong> to generate a <strong>setup token</strong>.</li>
                  <li>Paste the token above and click <strong>Connect</strong>. Tokens are single-use — if connecting fails, generate a fresh one.</li>
                </ol>
                <p class="text-muted">
                  TrackMyFI polls SimpleFIN at most once a day, per their guidelines. Bank
                  connections occasionally break and need to be re-established on the
                  SimpleFIN site; when that happens the gap is backfilled automatically on
                  the next successful sync.
                </p>
              </div>
            </template>
          </UAccordion>
        </template>

        <!-- Connected -->
        <template v-else>
          <div class="text-sm">
            Status: <span class="font-medium">Connected</span>
            · last synced {{ lastSynced }}
            <span v-if="linkedCount"> · {{ linkedCount }} linked {{ linkedCount === 1 ? 'account' : 'accounts' }}</span>
            <span v-if="status?.lastError" class="text-error"> · {{ status.lastError }}</span>
          </div>

          <!-- Bank connections needing attention at the bridge -->
          <div
            v-if="status?.bridgeErrors.length"
            class="flex items-start gap-3 rounded-lg border border-warning bg-warning/10 px-4 py-3 text-sm"
          >
            <span class="i-ph-warning-duotone mt-0.5 shrink-0 text-warning text-base" />
            <div class="space-y-1">
              <p class="font-medium text-warning">SimpleFIN reports connection problems</p>
              <ul class="list-disc ml-4 text-muted">
                <li v-for="e in status.bridgeErrors" :key="e">{{ e }}</li>
              </ul>
              <p class="text-muted">
                Re-establish the connection at <span class="font-mono">bridge.simplefin.org</span>.
                Missed days are backfilled automatically once it's fixed.
              </p>
            </div>
          </div>

          <div v-if="status?.accounts.length" class="space-y-2">
            <h3 class="font-semibold text-sm">Accounts</h3>
            <div class="rounded-lg border border-default divide-y divide-default">
              <div
                v-for="remote in status.accounts"
                :key="remote.id"
                class="flex items-center gap-4 px-4 py-3 text-sm"
              >
                <div class="flex-1 min-w-0">
                  <div class="font-medium truncate">{{ remote.name }}</div>
                  <div class="text-muted text-xs">
                    <span v-if="remote.org">{{ remote.org }} · </span>
                    <span class="tabular-nums">{{ money(remote.balance) }}</span>
                    as of {{ remote.balanceDate }}
                  </div>
                </div>
                <USelect
                  :model-value="linkValue(remote)"
                  :items="linkItems"
                  :loading="linkBusy === remote.id"
                  class="w-56"
                  @update:model-value="(v: string) => onLinkChange(remote, v)"
                />
              </div>
            </div>
            <p v-if="hasUnlinked" class="text-xs text-muted">
              Only linked accounts are imported. After linking, click
              <strong>Sync now</strong> to pull in their history.
            </p>
          </div>
          <p v-else class="text-sm text-muted">
            No accounts reported yet — connect your banks at
            <span class="font-mono">bridge.simplefin.org</span>, then click Sync now.
          </p>

          <div class="flex gap-2">
            <UFieldGroup>
              <UButton :loading="busy" :disabled="busy" @click="syncNow">Sync now</UButton>
              <UDropdownMenu :items="syncMenuItems" :content="{ align: 'end' }">
                <UButton
                  type="button"
                  color="primary"
                  icon="i-ph-caret-down"
                  aria-label="More sync options"
                  :disabled="busy"
                />
              </UDropdownMenu>
            </UFieldGroup>
            <UButton v-if="linkedCount" variant="soft" @click="reviewOpen = true">
              Review possible duplicates
            </UButton>
            <UButton color="error" variant="soft" :loading="busy" @click="disconnect">
              Disconnect
            </UButton>
          </div>
          <p v-if="linkedCount" class="text-xs text-muted">
            Tracked an account by hand before linking it? Bank sync may have imported
            transactions you already entered — use the duplicate review to clean them up.
          </p>

          <SimpleFinDuplicateReview v-model:open="reviewOpen" @resolved="accountsStore.loadList()" />

          <UModal v-model:open="rangeOpen" title="Sync a custom date range">
            <template #body>
              <div class="space-y-4">
                <p class="text-sm text-muted">
                  Re-fetches transactions your banks posted in this window and imports
                  anything that's missing — useful when a pending charge took long enough
                  to post that the normal sync window slid past it. Already-imported
                  transactions are never duplicated.
                </p>
                <div class="flex gap-4">
                  <UFormField label="From" class="flex-1">
                    <DateInput v-model="rangeStart" />
                  </UFormField>
                  <UFormField label="To" class="flex-1">
                    <DateInput v-model="rangeEnd" />
                  </UFormField>
                </div>
                <p v-if="rangeError" class="text-sm text-error" aria-live="polite">
                  {{ rangeError }}
                </p>
              </div>
            </template>
            <template #footer>
              <div class="flex justify-end gap-2 w-full">
                <UButton color="neutral" variant="soft" :disabled="busy" @click="rangeOpen = false">
                  Cancel
                </UButton>
                <UButton :loading="busy" :disabled="!!rangeError" @click="syncRange">
                  Sync range
                </UButton>
              </div>
            </template>
          </UModal>
        </template>

        <p v-if="message" class="text-sm text-error" aria-live="polite">{{ message }}</p>
      </section>
    </div>
  </div>
</template>
