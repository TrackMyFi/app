<script setup lang="ts">
import UInputNumber from '@nuxt/ui/components/InputNumber.vue'

const props = defineProps<{
  modelValue: number | null
  placeholder?: string
  min?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UInputNumber
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
