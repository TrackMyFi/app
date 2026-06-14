import { invoke } from '@tauri-apps/api/core'
import type { DeletionRange } from '../types/DeletionRange'
import type { DeletionPreview } from '../types/DeletionPreview'

export const previewDataDeletion = (range: DeletionRange) =>
  invoke<DeletionPreview>('preview_data_deletion', { range })

export const deleteData = (range: DeletionRange, resetProfile: boolean) =>
  invoke<void>('delete_data', { range, resetProfile })
