<script setup lang="ts">
defineProps<{ message: string }>()
defineEmits<{ retry: [] }>()
</script>

<template>
  <div class="flex flex-col items-center justify-center gap-3 rounded-lg border border-error/30 bg-error/5 px-6 py-10 text-center">
    <UIcon name="i-ph-warning-circle" class="size-8 text-error" />
    <div>
      <p class="font-medium">Couldn't load this page</p>
      <p class="text-sm text-muted mt-1 max-w-md break-words">{{ message }}</p>
    </div>
    <UButton color="neutral" variant="subtle" icon="i-ph-arrow-clockwise" @click="$emit('retry')">
      Try again
    </UButton>
  </div>
</template>
