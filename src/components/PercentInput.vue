<script setup lang="ts">
import UInputNumber from '@nuxt/ui/components/InputNumber.vue'

const props = defineProps<{
  modelValue: number | null
  min?: number
  max?: number
  /** Spinner increment as a fraction (0.01 = 1%). */
  step?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UInputNumber
    :model-value="props.modelValue"
    :format-options="{ style: 'percent', maximumFractionDigits: 2 }"
    :step="props.step ?? 0.01"
    :min="props.min ?? 0"
    :max="props.max"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
