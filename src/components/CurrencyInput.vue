<!-- src/components/CurrencyInput.vue -->
<script setup lang="ts">
const props = defineProps<{
  modelValue: number | null
  placeholder?: string
  min?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UNumberInput
    :model-value="props.modelValue"
    :format-options="{ style: 'currency', currency: 'USD' }"
    :step="0.01"
    :min="props.min"
    :placeholder="props.placeholder"
    :increment="false"
    :decrement="false"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
