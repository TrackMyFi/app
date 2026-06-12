import { invoke } from '@tauri-apps/api/core'
import type { FireProfile } from '../types/FireProfile'

export const getFireProfile = () => invoke<FireProfile>('get_fire_profile')
export const upsertFireProfile = (profile: FireProfile) =>
  invoke<void>('upsert_fire_profile', { profile })
