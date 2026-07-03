<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useToast } from '@nuxt/ui/composables'
import { listSimpleFinDuplicates } from '../lib/api/simplefin'
import { deleteTransaction, deleteTransactionKeepSnapshot } from '../lib/api/transactions'
import type { SimpleFinDuplicateCandidate } from '../lib/types/SimpleFinDuplicateCandidate'

const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{ resolved: [count: number] }>()

const toast = useToast()
const loading = ref(false)
const submitting = ref(false)
const candidates = ref<SimpleFinDuplicateCandidate[]>([])
// Keyed by simplefinTxnId (unique per pair after the backend's 1:1 matching).
const checked = ref<Record<number, boolean>>({})

watch(open, (v) => {
  if (v) load()
})

async function load() {
  loading.value = true
  try {
    candidates.value = await listSimpleFinDuplicates()
    const map: Record<number, boolean> = {}
    for (const c of candidates.value) {
      // Ordinary + contribution pairs default to checked. Net-deposit pairs
      // are higher stakes — resolving one deletes a paycheck's recorded
      // deposit transaction — so the user opts in per row.
      map[c.simplefinTxnId] = c.bucket !== 'net_deposit'
    }
    checked.value = map
  } catch (e) {
    toast.add({ title: String(e), color: 'error' })
    open.value = false
  } finally {
    loading.value = false
  }
}

const ordinary = computed(() => candidates.value.filter((c) => c.bucket === 'ordinary'))
const paycheckLinked = computed(() => candidates.value.filter((c) => c.bucket !== 'ordinary'))
const selected = computed(() => candidates.value.filter((c) => checked.value[c.simplefinTxnId]))

const money = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
const shortDate = (d: string) => DateTime.fromISO(d).toLocaleString(DateTime.DATE_MED)

const sourceLabel: Record<string, string> = {
  manual: 'manual entry',
  csv: 'CSV import',
  paycheck: 'paycheck',
}

async function submit() {
  submitting.value = true
  try {
    await Promise.all(
      selected.value.map((c) =>
        c.bucket === 'contribution'
          ? // Keep the paycheck-generated row (it carries is_contribution +
            // paycheck_id, which a SimpleFIN row never can) and delete the
            // SimpleFIN one. That row never owns balance snapshots, so the
            // normal delete path is correct for it.
            deleteTransaction(c.simplefinTxnId)
          : // Keep the SimpleFIN row (future syncs dedupe against it) and
            // delete the other side — but leave its generated balance
            // snapshot in place as a manual anchor, since SimpleFIN never
            // backfills historical balances.
            deleteTransactionKeepSnapshot(c.otherTxnId),
      ),
    )
    const n = selected.value.length
    toast.add({
      title: `Removed ${n} duplicate ${n === 1 ? 'transaction' : 'transactions'}`,
      color: 'success',
    })
    emit('resolved', n)
    open.value = false
  } catch (e) {
    toast.add({ title: String(e), color: 'error' })
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <UModal v-model:open="open" title="Review possible duplicates" :ui="{ content: 'max-w-3xl' }">
    <template #body>
      <div v-if="loading" class="flex items-center gap-2 text-sm text-muted py-4">
        <UIcon name="i-ph-circle-notch" class="animate-spin" />
        Looking for duplicates…
      </div>

      <div v-else-if="!candidates.length" class="text-sm text-muted py-4">
        No possible duplicates found. Bank-imported transactions were compared against
        your manual and CSV entries by account, amount, and date.
      </div>

      <div v-else class="space-y-5">
        <p class="text-sm text-muted">
          Each pair below looks like the same real-world transaction recorded twice —
          once by bank sync and once by hand or CSV import. Descriptions rarely match
          between the two, so compare them yourself and uncheck anything that's actually
          two separate transactions. Checked pairs are collapsed to a single row on submit.
        </p>

        <div v-if="ordinary.length" class="rounded-lg border border-default divide-y divide-default">
          <div
            v-for="c in ordinary"
            :key="c.simplefinTxnId"
            class="flex items-start gap-3 px-4 py-3 text-sm"
          >
            <UCheckbox v-model="checked[c.simplefinTxnId]" class="mt-0.5" />
            <div class="flex-1 min-w-0 space-y-0.5">
              <div class="flex items-baseline gap-2">
                <span class="font-medium truncate">{{ c.simplefinDescription }}</span>
                <span class="text-xs text-muted shrink-0">bank · {{ shortDate(c.simplefinDate) }}</span>
              </div>
              <div class="flex items-baseline gap-2 text-muted">
                <span class="truncate">{{ c.otherDescription }}</span>
                <span class="text-xs shrink-0">
                  {{ sourceLabel[c.otherImportSource] ?? c.otherImportSource }} · {{ shortDate(c.otherDate) }}
                </span>
              </div>
              <div class="text-xs text-muted">{{ c.accountName }}</div>
            </div>
            <span class="tabular-nums shrink-0" :class="c.txnType === 'income' ? 'text-success' : ''">
              {{ c.txnType === 'expense' ? '−' : '' }}{{ money(c.amount) }}
            </span>
          </div>
        </div>

        <div v-if="paycheckLinked.length" class="space-y-2">
          <div class="flex items-start gap-2 rounded-lg border border-warning bg-warning/10 px-3 py-2 text-xs">
            <span class="i-ph-warning-duotone mt-0.5 shrink-0 text-warning text-sm" />
            <p class="text-muted">
              These pairs involve transactions generated by a recorded paycheck.
              Contribution rows keep the paycheck side (the bank copy is removed) so
              contribution tracking stays intact. Deposit rows start unchecked — resolving
              one removes the paycheck's recorded deposit and keeps the bank copy.
            </p>
          </div>
          <div class="rounded-lg border border-warning/50 divide-y divide-default">
            <div
              v-for="c in paycheckLinked"
              :key="c.simplefinTxnId"
              class="flex items-start gap-3 px-4 py-3 text-sm"
            >
              <UCheckbox v-model="checked[c.simplefinTxnId]" class="mt-0.5" />
              <div class="flex-1 min-w-0 space-y-0.5">
                <div class="flex items-baseline gap-2">
                  <span class="font-medium truncate">{{ c.simplefinDescription }}</span>
                  <span class="text-xs text-muted shrink-0">bank · {{ shortDate(c.simplefinDate) }}</span>
                </div>
                <div class="flex items-baseline gap-2 text-muted">
                  <span class="truncate">{{ c.otherDescription }}</span>
                  <span class="text-xs shrink-0">paycheck · {{ shortDate(c.otherDate) }}</span>
                </div>
                <div class="flex items-center gap-2 text-xs">
                  <span class="text-muted">{{ c.accountName }}</span>
                  <UBadge
                    :color="c.bucket === 'contribution' ? 'info' : 'warning'"
                    variant="soft"
                    size="sm"
                  >
                    {{ c.bucket === 'contribution' ? 'contribution — keeps paycheck row' : 'paycheck deposit' }}
                  </UBadge>
                </div>
              </div>
              <span class="tabular-nums shrink-0 text-success">{{ money(c.amount) }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <template #footer>
      <div class="flex justify-between items-center w-full">
        <p class="text-xs text-muted">
          <template v-if="candidates.length">
            {{ selected.length }} of {{ candidates.length }}
            {{ candidates.length === 1 ? 'pair' : 'pairs' }} selected
          </template>
        </p>
        <div class="flex gap-2">
          <UButton variant="ghost" :disabled="submitting" @click="open = false">Cancel</UButton>
          <UButton
            v-if="candidates.length"
            :disabled="!selected.length"
            :loading="submitting"
            @click="submit"
          >
            Remove duplicates
          </UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>
