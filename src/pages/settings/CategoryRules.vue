<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useToast } from '@nuxt/ui/composables'
import { DateTime } from 'luxon'
import * as categoryRulesApi from '../../lib/api/categoryRules'
import type { CategoryRule } from '../../lib/types/CategoryRule'
import { categoryItems, labelForCategory } from '../../lib/transactions/constants'
import PageError from '../../components/PageError.vue'
import SettingsNav from '../../components/SettingsNav.vue'
import { usePageData } from '../../composables/usePageData'

const toast = useToast()
const { error, run, retry } = usePageData()

const categoryRules = ref<CategoryRule[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
const savingRule = ref(false)
const removingRuleId = ref<number | null>(null)

onMounted(() => run(async () => {
  categoryRules.value = await categoryRulesApi.listCategoryRules()
}))

async function addCategoryRule() {
  if (!newRuleKeyword.value.trim()) return
  savingRule.value = true
  try {
    await categoryRulesApi.createCategoryRule(
      newRuleKeyword.value.trim().toLowerCase(),
      newRuleCategory.value,
      DateTime.now().toISO()!,
    )
    categoryRules.value = await categoryRulesApi.listCategoryRules()
    newRuleKeyword.value = ''
    newRuleCategory.value = 'discretionary'
  } catch (err) {
    toast.add({ title: 'Failed to add rule', description: String(err), color: 'error' })
  } finally {
    savingRule.value = false
  }
}

async function removeCategoryRule(id: number) {
  removingRuleId.value = id
  try {
    await categoryRulesApi.deleteCategoryRule(id)
    categoryRules.value = await categoryRulesApi.listCategoryRules()
  } catch (err) {
    toast.add({ title: 'Failed to delete rule', description: String(err), color: 'error' })
  } finally {
    removingRuleId.value = null
  }
}

const ruleColumns = [
  { accessorKey: 'keyword', header: 'Keyword' },
  { accessorKey: 'category', header: 'Category', cell: ({ row }: { row: { original: { category: string } } }) => labelForCategory(row.original.category) },
  { id: 'actions', header: '', meta: { class: { td: 'text-right' } } },
]
</script>

<template>
  <div class="p-6 max-w-3xl">
    <h1 class="text-2xl font-bold mb-4">Settings</h1>
    <SettingsNav />

    <PageError v-if="error" :message="error" @retry="retry" />

    <section v-else class="space-y-3">
      <h2 class="text-xl font-bold">Category Rules</h2>
      <p class="text-sm text-muted">
        Keywords matched against transaction descriptions during CSV import.
        First matching rule wins; unmatched rows use the mapping's default category.
      </p>

      <UTable :data="categoryRules" :columns="ruleColumns" empty="No rules yet.">
        <template #keyword-cell="{ row }">
          <span class="font-mono text-xs">{{ row.original.keyword }}</span>
        </template>
        <template #actions-cell="{ row }">
          <UButton size="xs" color="error" variant="ghost" :loading="removingRuleId === row.original.id" :disabled="removingRuleId !== null" @click="removeCategoryRule(row.original.id)">
            Remove
          </UButton>
        </template>
      </UTable>

      <div class="flex gap-2 items-center pt-1">
        <UInput
          v-model="newRuleKeyword"
          placeholder="keyword (e.g. netflix)"
          class="flex-1"
          @keydown.enter="addCategoryRule"
        />
        <USelect v-model="newRuleCategory" :items="categoryItems" class="w-44" />
        <UButton size="sm" variant="soft" :loading="savingRule" :disabled="!newRuleKeyword.trim() || savingRule" @click="addCategoryRule">
          Add rule
        </UButton>
      </div>
    </section>
  </div>
</template>
