<script setup lang="ts">
import { useRoute } from 'vue-router'

const route = useRoute()

const tabs = [
  { label: 'FIRE Profile', to: '/settings/profile' },
  { label: 'Category Rules', to: '/settings/category-rules' },
  { label: 'Vendor Rules', to: '/settings/vendor-rules' },
  { label: 'Bank Sync', to: '/settings/bank-sync' },
  { label: 'Data & Sync', to: '/settings/sync' },
  { label: 'General', to: '/settings/general' },
]

function isActive(to: string) {
  return route.path === to
}
</script>

<template>
  <nav class="flex items-center gap-1 border-b border-default mb-6 -mt-1">
    <RouterLink
      v-for="t in tabs"
      :key="t.to"
      :to="t.to"
      :class="[
        'px-3 py-2 text-sm rounded-t-md border-b-2 -mb-px',
        isActive(t.to)
          ? 'border-primary text-primary font-medium'
          : 'border-transparent text-muted hover:text-default',
      ]"
    >
      {{ t.label }}
    </RouterLink>
  </nav>
</template>
