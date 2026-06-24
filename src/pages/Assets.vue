<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useAssetEventsStore } from '../stores/assetEvents'
import { useAccountsStore } from '../stores/accounts'
import { assetEventKindItems, labelForAssetEventKind, colorForAssetEventKind } from '../lib/assets/constants'
import {
  lifetimeCost,
  costBasisAdded,
  annualizedCost,
  currentValue,
  groupByAsset,
  type AssetGroup,
} from '../lib/assets/rollups'
import AssetEventForm from '../components/AssetEventForm.vue'
import DateInput from '../components/DateInput.vue'
import type { AssetEvent } from '../lib/types/AssetEvent'

const store = useAssetEventsStore()
const accountsStore = useAccountsStore()
const toast = useToast()

const isModalOpen = ref(false)
const editing = ref<AssetEvent | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(e: AssetEvent) { editing.value = e; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

watch(isModalOpen, (open) => { if (!open) editing.value = null })

const removingId = ref<number | null>(null)
async function removeRow(e: AssetEvent) {
  const ok = await confirm(`Delete "${e.description}" on ${e.date}?`, { title: 'Delete asset event' })
  if (!ok) return
  removingId.value = e.id
  try {
    await store.remove(e.id)
  } catch (err) {
    toast.add({ title: 'Failed to delete event', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
}

// ---- filters ----
// 'all' is a sentinel (Reka UI's <SelectItem> rejects an empty-string value).
const kindFilter = ref('all')
const startDate = ref('')
const endDate = ref('')
const search = ref('')

async function applyFilters() {
  await store.setFilter({
    kind: kindFilter.value === 'all' ? null : kindFilter.value,
    startDate: startDate.value || null,
    endDate: endDate.value || null,
    search: search.value || null,
  })
}
async function clearFilters() {
  kindFilter.value = 'all'
  startDate.value = ''
  endDate.value = ''
  search.value = ''
  await store.setFilter({ kind: null, startDate: null, endDate: null, search: null })
}

const kindItems = [{ label: 'All types', value: 'all' }, ...assetEventKindItems]

// ---- rollups ----
const events = computed(() => store.assetEvents)
const groups = computed(() => groupByAsset(events.value))

// Current value: real-estate assets use their account's latest balance snapshot;
// free-text assets use the last value recorded on one of their events.
function latestBalanceFor(accountId: number): number | null {
  return accountsStore.latestBalances.find((b) => b.accountId === accountId)?.balance ?? null
}
function assetValueForGroup(group: AssetGroup): number | null {
  return group.accountId != null ? latestBalanceFor(group.accountId) : currentValue(group.events)
}
const totalCurrentValue = computed(() =>
  groups.value.reduce((sum, g) => {
    const v = assetValueForGroup(g)
    return v != null ? sum + v : sum
  }, 0),
)

const totals = computed(() => ({
  spent: lifetimeCost(events.value),
  improvements: costBasisAdded(events.value),
  value: totalCurrentValue.value,
  annualized: annualizedCost(events.value),
}))

function assetName(accountId: number | null, label: string | null): string {
  if (accountId != null) {
    return accountsStore.accounts.find((a) => a.id === accountId)?.name ?? `Account #${accountId}`
  }
  return label || 'Unlabeled'
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
function fmtDate(iso: string): string {
  return DateTime.fromISO(iso).toLocaleString(DateTime.DATE_MED)
}

onMounted(async () => {
  await accountsStore.loadList()
  await store.load()
  kindFilter.value = store.filter.kind ?? 'all'
  startDate.value = store.filter.startDate ?? ''
  endDate.value = store.filter.endDate ?? ''
  search.value = store.filter.search ?? ''
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Assets</h1>
        <p class="text-sm text-muted">Upkeep, repairs &amp; improvements for the things you own.</p>
      </div>
      <UButton icon="i-ph-plus" @click="openAdd">Add event</UButton>
    </div>

    <!-- Rollup stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Total spent</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ money(totals.spent) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Current value</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ money(totals.value) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Improvements (added to basis)</p>
        <p class="text-xl font-semibold tabular-nums mt-1 text-success">{{ money(totals.improvements) }}</p>
      </div>
      <div class="border border-default rounded-lg p-4">
        <p class="text-xs uppercase tracking-wide text-muted">Annualized cost</p>
        <p class="text-xl font-semibold tabular-nums mt-1">{{ money(totals.annualized) }}</p>
      </div>
    </div>

    <!-- Filters -->
    <div class="flex flex-wrap gap-2 items-end">
      <div>
        <p class="text-xs text-muted mb-1">Type</p>
        <USelect v-model="kindFilter" :items="kindItems" class="w-40" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">From</p>
        <DateInput v-model="startDate" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">To</p>
        <DateInput v-model="endDate" />
      </div>
      <UInput v-model="search" placeholder="Search" class="w-44" />
      <UButton @click="applyFilters">Apply</UButton>
      <UButton variant="ghost" @click="clearFilters">Clear</UButton>
    </div>

    <p v-if="groups.length === 0" class="text-sm text-muted py-8 text-center">
      No asset events yet. Add your first roof, furnace, or car service above.
    </p>

    <!-- Grouped by asset -->
    <div v-for="group in groups" :key="group.key" class="border border-default rounded-lg overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 bg-muted/50">
        <div class="flex items-center gap-2">
          <UIcon :name="group.accountId != null ? 'i-ph-house' : 'i-ph-tag'" class="text-muted" />
          <span class="font-medium">{{ assetName(group.accountId, group.label) }}</span>
        </div>
        <div class="text-sm text-muted tabular-nums">
          <span>{{ money(lifetimeCost(group.events)) }} spent</span>
          <span v-if="assetValueForGroup(group) != null" class="ml-3">
            {{ money(assetValueForGroup(group)!) }} value
          </span>
          <span v-if="costBasisAdded(group.events) > 0" class="ml-3 text-success">
            {{ money(costBasisAdded(group.events)) }} basis
          </span>
        </div>
      </div>
      <table class="w-full text-sm">
        <tbody>
          <tr v-for="e in group.events" :key="e.id" class="border-t border-default">
            <td class="px-4 py-2 whitespace-nowrap text-muted">{{ fmtDate(e.date) }}</td>
            <td class="px-4 py-2">
              <UBadge :color="colorForAssetEventKind(e.kind)" variant="subtle" size="xs">
                {{ labelForAssetEventKind(e.kind) }}
              </UBadge>
            </td>
            <td class="px-4 py-2">
              <span>{{ e.description }}</span>
              <span v-if="e.vendor" class="text-muted"> · {{ e.vendor }}</span>
            </td>
            <td class="px-4 py-2 text-right tabular-nums">{{ money(e.cost) }}</td>
            <td class="px-4 py-2 text-right whitespace-nowrap">
              <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="openEdit(e)" />
              <UButton
                size="xs"
                variant="ghost"
                color="error"
                icon="i-ph-trash"
                :loading="removingId === e.id"
                :disabled="removingId !== null"
                @click="removeRow(e)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit asset event' : 'Add asset event'" class="sm:w-[560px] max-w-full">
      <template #body>
        <AssetEventForm :editing="editing" @saved="onSaved" />
      </template>
    </UModal>
  </div>
</template>
