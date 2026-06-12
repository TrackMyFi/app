import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { FireProfile } from '../lib/types/FireProfile'
import { getFireProfile, upsertFireProfile } from '../lib/api/fireProfile'

export const useFireProfileStore = defineStore('fireProfile', () => {
  const profile = ref<FireProfile | null>(null)
  async function load() { profile.value = await getFireProfile() }
  async function save(next: FireProfile) {
    await upsertFireProfile(next)
    profile.value = next
  }
  return { profile, load, save }
})
