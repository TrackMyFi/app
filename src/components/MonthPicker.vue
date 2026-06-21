<script setup lang="ts">
import { ref, computed } from 'vue'
import { DateTime } from 'luxon'
import { CalendarDate, type DateValue } from '@internationalized/date'

const props = defineProps<{
  modelValue: DateTime
  mode?: 'month' | 'year'
}>()

const emit = defineEmits<{
  'update:modelValue': [value: DateTime]
}>()

const isOpen = ref(false)

const label = computed(() =>
  props.mode === 'year'
    ? props.modelValue.toFormat('yyyy')
    : props.modelValue.toFormat('MMMM yyyy')
)

const calValue = computed<DateValue>({
  get() {
    return new CalendarDate(props.modelValue.year, props.modelValue.month, 1)
  },
  set(v) {
    isOpen.value = false
    emit('update:modelValue', DateTime.fromObject({ year: v.year, month: v.month, day: 1 }).startOf('month'))
  },
})

function prev() {
  const dt = props.mode === 'year'
    ? props.modelValue.minus({ years: 1 })
    : props.modelValue.minus({ months: 1 })
  emit('update:modelValue', dt.startOf('month'))
}

function next() {
  const dt = props.mode === 'year'
    ? props.modelValue.plus({ years: 1 })
    : props.modelValue.plus({ months: 1 })
  emit('update:modelValue', dt.startOf('month'))
}
</script>

<template>
  <div class="flex items-center gap-1">
    <UButton variant="ghost" color="neutral" icon="i-ph-caret-left" @click="prev" />
    <UPopover v-model:open="isOpen">
      <UButton variant="ghost" color="neutral">
        {{ label }}
      </UButton>
      <template #content>
        <div class="p-2">
          <UCalendar v-model="calValue" :type="mode === 'year' ? 'year' : 'month'" />
        </div>
      </template>
    </UPopover>
    <UButton variant="ghost" color="neutral" icon="i-ph-caret-right" @click="next" />
  </div>
</template>
