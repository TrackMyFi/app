<script setup lang="ts">
import { ref, computed } from 'vue'
import { DateTime } from 'luxon'
import { CalendarDate, type DateValue } from '@internationalized/date'

const props = defineProps<{
  modelValue: DateTime
}>()

const emit = defineEmits<{
  'update:modelValue': [value: DateTime]
}>()

const isOpen = ref(false)

const monthLabel = computed(() => props.modelValue.toFormat('MMMM yyyy'))

const calValue = computed<DateValue>({
  get() {
    return new CalendarDate(props.modelValue.year, props.modelValue.month, 1)
  },
  set(v) {
    isOpen.value = false
    emit('update:modelValue', DateTime.fromObject({ year: v.year, month: v.month, day: 1 }).startOf('month'))
  },
})

function prevMonth() {
  emit('update:modelValue', props.modelValue.minus({ months: 1 }).startOf('month'))
}

function nextMonth() {
  emit('update:modelValue', props.modelValue.plus({ months: 1 }).startOf('month'))
}
</script>

<template>
  <div class="flex items-center gap-1">
    <UButton variant="ghost" color="neutral" icon="i-ph-caret-left" @click="prevMonth" />
    <UPopover v-model:open="isOpen">
      <UButton variant="ghost" color="neutral">
        {{ monthLabel }}
      </UButton>
      <template #content>
        <div class="p-2">
          <UCalendar v-model="calValue" type="month" />
        </div>
      </template>
    </UPopover>
    <UButton variant="ghost" color="neutral" icon="i-ph-caret-right" @click="nextMonth" />
  </div>
</template>
