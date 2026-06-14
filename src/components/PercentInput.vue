<!-- src/components/PercentInput.vue -->
<script setup lang="ts">
const props = defineProps<{
  modelValue: number | null
  min?: number
  max?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UNumberInput
    :model-value="props.modelValue"
    :format-options="{ style: 'percent' }"
    :step="0.01"
    :min="props.min ?? 0"
    :max="props.max"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
