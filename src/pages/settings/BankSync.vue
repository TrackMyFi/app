<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useToast } from '@nuxt/ui/composables'
import { useSimpleFinStore } from '../../stores/simplefin'
import { useAccountsStore } from '../../stores/accounts'
import { createAccount } from '../../lib/api/accounts'
import type { SimpleFinRemoteAccount } from '../../lib/types/SimpleFinRemoteAccount'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'

const toast = useToast()
const { error, run, retry } = usePageData()

const simplefin = useSimpleFinStore()
const accountsStore = useAccountsStore()

const setupToken = ref('')
const busy = ref(false)
const message = ref('')
const linkBusy = ref<string | null>(null)

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
        createdAt: new Date().toISOString().slice(0, 10),
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

async function syncNow() {
  busy.value = true
  message.value = ''
  try {
    const s = await simplefin.syncNow()
    toast.add({
      title: `Synced ${s.accountsSynced} ${s.accountsSynced === 1 ? 'account' : 'accounts'}: ${s.transactionsAdded} new ${s.transactionsAdded === 1 ? 'transaction' : 'transactions'}, ${s.snapshotsAdded} balance ${s.snapshotsAdded === 1 ? 'snapshot' : 'snapshots'}`,
      color: 'success',
    })
    await accountsStore.loadList()
  } catch (e) {
    message.value = String(e)
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
  <div class="p-6 max-w-3xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <div v-else class="space-y-8">
      <section class="space-y-3">
        <h2 class="text-xl font-bold">Bank Sync (SimpleFIN)</h2>
        <p class="text-sm text-muted">
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
            <UButton :loading="busy" @click="syncNow">Sync now</UButton>
            <UButton color="error" variant="soft" :loading="busy" @click="disconnect">
              Disconnect
            </UButton>
          </div>
        </template>

        <p v-if="message" class="text-sm text-error" aria-live="polite">{{ message }}</p>
      </section>
    </div>
  </div>
</template>
