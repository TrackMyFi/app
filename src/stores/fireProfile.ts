import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FireProfile } from '../lib/types/FireProfile'
import { getFireProfile, upsertFireProfile } from '../lib/api/fireProfile'

export const useFireProfileStore = defineStore('fireProfile', () => {
  const profile = ref<FireProfile | null>(null)
  async function load() { profile.value = await getFireProfile() }
  async function save(next: FireProfile) {
    await upsertFireProfile(next)
    profile.value = next
  }

  const currentAge = computed(() => {
    if (!profile.value?.dateOfBirth) return 0
    const today = new Date()
    const birth = new Date(profile.value.dateOfBirth)
    let age = today.getFullYear() - birth.getFullYear()
    const m = today.getMonth() - birth.getMonth()
    if (m < 0 || (m === 0 && today.getDate() < birth.getDate())) age--
    return age
  })

  return { profile, load, save, currentAge }
})
