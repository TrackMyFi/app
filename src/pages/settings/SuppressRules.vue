<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../../stores/accounts'
import * as suppressRulesApi from '../../lib/api/suppressRules'
import type { SuppressRule } from '../../lib/types/SuppressRule'
import { suppressKindItems, labelForSuppressKind } from '../../lib/transactions/constants'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'

const toast = useToast()
const { error, run, retry } = usePageData()
const accountsStore = useAccountsStore()

const rules = ref<SuppressRule[]>([])
const newKeyword = ref('')
const newKind = ref('investment_activity')
const ALL_ACCOUNTS = 0
const newAccountId = ref<number>(ALL_ACCOUNTS)
const saving = ref(false)
const removingId = ref<number | null>(null)

const editingId = ref<number | null>(null)
const editKeyword = ref('')
const editKind = ref('investment_activity')
const editAccountId = ref<number>(ALL_ACCOUNTS)
const savingEditId = ref<number | null>(null)

onMounted(() => run(async () => {
  rules.value = await suppressRulesApi.listSuppressRules()
  await accountsStore.load()
}))

const accountItems = computed(() => [
  { label: 'All accounts', value: ALL_ACCOUNTS },
  ...accountsStore.accounts.map((a) => ({ label: a.name, value: a.id })),
])

function accountName(id: number | null): string {
  if (id == null) return 'All accounts'
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

async function addRule() {
  if (!newKeyword.value.trim()) return
  saving.value = true
  try {
    const [, suppressedCount] = await suppressRulesApi.createSuppressRule(
      newKeyword.value.trim().toLowerCase(),
      newKind.value,
      newAccountId.value === ALL_ACCOUNTS ? null : newAccountId.value,
      DateTime.now().toISO()!,
    )
    rules.value = await suppressRulesApi.listSuppressRules()
    newKeyword.value = ''
    toast.add({
      title: 'Rule added',
      description: `${suppressedCount} transaction${suppressedCount === 1 ? '' : 's'} now suppressed in total.`,
      color: 'success',
    })
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    saving.value = false
  }
}

async function removeRule(id: number) {
  removingId.value = id
  try {
    await suppressRulesApi.deleteSuppressRule(id)
    rules.value = await suppressRulesApi.listSuppressRules()
  } catch (err) {
    toast.add({ title: 'Failed to delete rule', description: String(err), color: 'error' })
  } finally {
    removingId.value = null
  }
}

function startEdit(rule: SuppressRule) {
  editingId.value = rule.id
  editKeyword.value = rule.keyword
  editKind.value = rule.kind
  editAccountId.value = rule.accountId ?? ALL_ACCOUNTS
}

function cancelEdit() {
  editingId.value = null
}

async function saveEdit(id: number) {
  if (!editKeyword.value.trim()) return
  savingEditId.value = id
  try {
    await suppressRulesApi.updateSuppressRule(
      id,
      editKeyword.value.trim().toLowerCase(),
      editKind.value,
      editAccountId.value === ALL_ACCOUNTS ? null : editAccountId.value,
    )
    rules.value = await suppressRulesApi.listSuppressRules()
    editingId.value = null
  } catch (err) {
    toast.add({ title: 'Failed to update rule', description: String(err), color: 'error' })
  } finally {
    savingEditId.value = null
  }
}

const columns = [
  { accessorKey: 'keyword', header: 'Keyword' },
  { id: 'kind', header: 'Suppress as' },
  { id: 'account', header: 'Account' },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]
</script>

<template>
  <div class="p-6 max-w-3xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <section v-else class="space-y-3">
      <h2 class="text-xl font-bold">Suppress Rules</h2>
      <p class="text-sm text-muted">
        Keywords matched against transaction descriptions (first matching rule
        wins). Matches are kept in the database — they still count toward account
        balances — but are hidden from the transactions list, totals, and charts.
        Useful for bank-sync noise like 401(k) fees or realized gain/loss entries.
        Scope a rule to one account so "fees" inside an investment account never
        hides a real fee on checking.
      </p>

      <UTable :data="rules" :columns="columns" empty="No rules yet.">
        <template #keyword-cell="{ row }">
          <UInput
            v-if="editingId === row.original.id"
            v-model="editKeyword"
            size="xs"
            class="font-mono"
            @keydown.enter="saveEdit(row.original.id)"
            @keydown.escape="cancelEdit"
          />
          <span v-else class="font-mono text-xs">{{ row.original.keyword }}</span>
        </template>
        <template #kind-cell="{ row }">
          <USelect
            v-if="editingId === row.original.id"
            v-model="editKind"
            :items="suppressKindItems"
            value-key="value"
            size="xs"
          />
          <span v-else>{{ labelForSuppressKind(row.original.kind) }}</span>
        </template>
        <template #account-cell="{ row }">
          <USelect
            v-if="editingId === row.original.id"
            v-model="editAccountId"
            :items="accountItems"
            value-key="value"
            size="xs"
          />
          <span v-else :class="row.original.accountId == null ? 'text-muted' : ''">
            {{ accountName(row.original.accountId) }}
          </span>
        </template>
        <template #actions-cell="{ row }">
          <div v-if="editingId === row.original.id" class="flex gap-2 justify-end">
            <UButton size="xs" variant="ghost" color="neutral" :disabled="savingEditId !== null" @click="cancelEdit">
              Cancel
            </UButton>
            <UButton size="xs" variant="soft" :loading="savingEditId === row.original.id" :disabled="!editKeyword.trim() || savingEditId !== null" @click="saveEdit(row.original.id)">
              Save
            </UButton>
          </div>
          <div v-else class="flex gap-2 justify-end">
            <UButton size="xs" variant="ghost" :disabled="editingId !== null || removingId !== null" @click="startEdit(row.original)">
              Edit
            </UButton>
            <UButton size="xs" color="error" variant="ghost" :loading="removingId === row.original.id" :disabled="removingId !== null || editingId !== null" @click="removeRule(row.original.id)">
              Remove
            </UButton>
          </div>
        </template>
      </UTable>

      <div class="flex gap-2 items-center pt-1">
        <UInput
          v-model="newKeyword"
          placeholder="keyword (e.g. realizedgainloss)"
          class="flex-1"
          @keydown.enter="addRule"
        />
        <USelect
          v-model="newKind"
          :items="suppressKindItems"
          value-key="value"
          class="w-44"
        />
        <USelect
          v-model="newAccountId"
          :items="accountItems"
          value-key="value"
          class="w-44"
        />
        <UButton size="sm" variant="soft" :loading="saving" :disabled="!newKeyword.trim() || saving" @click="addRule">
          Add rule
        </UButton>
      </div>
    </section>
  </div>
</template>
