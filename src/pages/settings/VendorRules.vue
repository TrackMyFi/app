<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../../stores/accounts'
import * as vendorRulesApi from '../../lib/api/vendorRules'
import * as transactionsApi from '../../lib/api/transactions'
import type { VendorRule } from '../../lib/types/VendorRule'
import type { Transaction } from '../../lib/types/Transaction'
import type { VendorRuleInput } from '../../lib/expenses/merchants'
import { suggestVendorRules, type VendorRuleSuggestion } from '../../lib/expenses/vendorSuggestions'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'

const toast = useToast()
const { error, run, retry } = usePageData()
const accountsStore = useAccountsStore()

const vendorRules = ref<VendorRule[]>([])
const newVendorKeyword = ref('')
const newVendorName = ref('')
const savingVendorRule = ref(false)
const removingVendorRuleId = ref<number | null>(null)

// ---- Vendor rule suggestions (from the user's own transaction history) ----
const suggestionTransactions = ref<Transaction[]>([])
const dismissedSuggestionKeys = ref<Set<string>>(new Set())
const addingSuggestionKey = ref<string | null>(null)

onMounted(() => run(async () => {
  vendorRules.value = await vendorRulesApi.listVendorRules()
  await accountsStore.load()
  suggestionTransactions.value = (await transactionsApi.listTransactions({ limit: null })).rows
}))

const vendorRuleInputs = computed<VendorRuleInput[]>(() =>
  vendorRules.value.map((r) => ({ keyword: r.keyword, vendorName: r.vendorName })),
)

const vendorRuleSuggestions = computed(() =>
  suggestVendorRules(suggestionTransactions.value, accountsStore.accounts, vendorRuleInputs.value)
    .filter((s) => !dismissedSuggestionKeys.value.has(s.key)),
)

async function acceptVendorRuleSuggestion(s: VendorRuleSuggestion) {
  addingSuggestionKey.value = s.key
  try {
    await vendorRulesApi.createVendorRule(s.keyword, s.vendorName, DateTime.now().toISO()!)
    vendorRules.value = await vendorRulesApi.listVendorRules()
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    addingSuggestionKey.value = null
  }
}

function dismissVendorRuleSuggestion(s: VendorRuleSuggestion) {
  dismissedSuggestionKeys.value = new Set(dismissedSuggestionKeys.value).add(s.key)
}

async function addVendorRule() {
  if (!newVendorKeyword.value.trim() || !newVendorName.value.trim()) return
  savingVendorRule.value = true
  try {
    await vendorRulesApi.createVendorRule(
      newVendorKeyword.value.trim().toLowerCase(),
      newVendorName.value.trim(),
      DateTime.now().toISO()!,
    )
    vendorRules.value = await vendorRulesApi.listVendorRules()
    newVendorKeyword.value = ''
    newVendorName.value = ''
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    savingVendorRule.value = false
  }
}

async function removeVendorRule(id: number) {
  removingVendorRuleId.value = id
  try {
    await vendorRulesApi.deleteVendorRule(id)
    vendorRules.value = await vendorRulesApi.listVendorRules()
  } catch (err) {
    toast.add({ title: 'Failed to delete rule', description: String(err), color: 'error' })
  } finally {
    removingVendorRuleId.value = null
  }
}

const vendorRuleColumns = [
  { accessorKey: 'keyword', header: 'Keyword' },
  { accessorKey: 'vendorName', header: 'Vendor name' },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
</script>

<template>
  <div class="p-6 max-w-3xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <section v-else class="space-y-3">
      <h2 class="text-xl font-bold">Vendor Rules</h2>
      <p class="text-sm text-muted">
        Keywords matched against transaction descriptions on the Expenses page.
        When a description contains the keyword, its vendor name is used instead
        of the auto-detected one — handy for messy or inconsistent bank descriptions.
        If more than one rule matches, the longest keyword wins.
      </p>

      <UTable :data="vendorRules" :columns="vendorRuleColumns" empty="No rules yet.">
        <template #keyword-cell="{ row }">
          <span class="font-mono text-xs">{{ row.original.keyword }}</span>
        </template>
        <template #actions-cell="{ row }">
          <UButton size="xs" color="error" variant="ghost" :loading="removingVendorRuleId === row.original.id" :disabled="removingVendorRuleId !== null" @click="removeVendorRule(row.original.id)">
            Remove
          </UButton>
        </template>
      </UTable>

      <div class="flex gap-2 items-center pt-1">
        <UInput
          v-model="newVendorKeyword"
          placeholder="keyword (e.g. pizza hut)"
          class="flex-1"
          @keydown.enter="addVendorRule"
        />
        <UInput
          v-model="newVendorName"
          placeholder="vendor name (e.g. Pizza Hut)"
          class="flex-1"
          @keydown.enter="addVendorRule"
        />
        <UButton size="sm" variant="soft" :loading="savingVendorRule" :disabled="!newVendorKeyword.trim() || !newVendorName.trim() || savingVendorRule" @click="addVendorRule">
          Add rule
        </UButton>
      </div>

      <div v-if="vendorRuleSuggestions.length" class="space-y-2 pt-3 mt-2 border-t border-default">
        <p class="text-xs font-medium text-muted uppercase tracking-wide">Suggested from your transactions</p>
        <div
          v-for="s in vendorRuleSuggestions"
          :key="s.key"
          class="flex items-center justify-between gap-3 rounded-lg border border-dashed border-default px-3 py-2"
        >
          <div class="min-w-0">
            <p class="text-sm text-heading">
              <span class="font-medium">{{ s.vendorName }}</span>
              <span class="text-xs text-dimmed"> — keyword "{{ s.keyword }}"</span>
            </p>
            <p class="text-xs text-muted truncate">
              {{ s.count }} transactions · {{ money(s.total) }} · e.g. {{ s.sampleDescriptions.join(', ') }}
            </p>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <UButton size="xs" variant="ghost" color="neutral" @click="dismissVendorRuleSuggestion(s)">Dismiss</UButton>
            <UButton
              size="xs"
              variant="soft"
              :loading="addingSuggestionKey === s.key"
              :disabled="addingSuggestionKey !== null"
              @click="acceptVendorRuleSuggestion(s)"
            >
              Add rule
            </UButton>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
