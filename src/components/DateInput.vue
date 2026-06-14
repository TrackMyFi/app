<script setup lang="ts">
import { computed } from 'vue'
import { parseDate, type DateValue } from '@internationalized/date'

/**
 * Reusable date input: a NuxtUI UInputDate (segmented keyboard entry) with a
 * calendar-picker popover, exposing a plain ISO date string (yyyy-MM-dd) via
 * v-model so callers don't deal with @internationalized/date types.
 */
const props = withDefaults(defineProps<{ modelValue: string }>(), { modelValue: '' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const dateValue = computed<DateValue | undefined>({
  get() {
    if (!props.modelValue) return undefined
    try {
      return parseDate(props.modelValue)
    } catch {
      return undefined
    }
  },
  set(value) {
    emit('update:modelValue', value ? value.toString() : '')
  },
})
</script>

<template>
  <UInputDate v-model="dateValue">
    <template #trailing>
      <UPopover>
        <UButton
          color="neutral"
          variant="link"
          size="sm"
          icon="i-ph-calendar"
          aria-label="Open calendar"
        />
        <template #content>
          <UCalendar v-model="dateValue" class="p-2" />
        </template>
      </UPopover>
    </template>
  </UInputDate>
</template>
