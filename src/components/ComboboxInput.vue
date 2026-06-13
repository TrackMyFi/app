<script setup lang="ts">
const model = defineModel<string>({ default: '' })
defineProps<{ items: string[]; placeholder?: string }>()
</script>

<template>
  <UInputMenu v-model="model" mode="autocomplete" :items="items" :placeholder="placeholder">
    <template #empty>
      <span v-if="model">Create "{{ model }}"</span>
      <span v-else>No options</span>
    </template>
  </UInputMenu>
</template>
