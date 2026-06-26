<script setup lang="ts">
import UInputNumber from '@nuxt/ui/components/InputNumber.vue'

const props = defineProps<{
  modelValue: number | null
  placeholder?: string
  min?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()

// Select the whole value when the field gains focus (tab or click) so the
// pre-populated $0.00 can be overtyped without first clearing it. Deferring to
// the next frame lets a click's mouseup finish before we select, otherwise the
// browser would collapse the selection to a caret.
function selectOnFocus(event: FocusEvent) {
  const input = event.target as HTMLInputElement | null
  if (input && typeof input.select === 'function') {
    requestAnimationFrame(() => input.select())
  }
}
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
    @focusin="selectOnFocus"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
