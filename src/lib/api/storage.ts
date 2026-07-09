import { invoke } from '@tauri-apps/api/core'
import type { AssetAttachment } from '../types/AssetAttachment'
import type { HsaAttachment } from '../types/HsaAttachment'
import type { MigrationSummary } from '../types/MigrationSummary'
import type { StorageInfo } from '../types/StorageInfo'

export type SaveStorageConfigArgs = {
  provider: string
  bucketName?: string | null
  r2AccountId?: string | null
  s3Region?: string | null
  accessKeyId?: string | null
  secretAccessKey?: string | null
  serviceAccountJson?: string | null
}

export function getStorageConfig(): Promise<StorageInfo> {
  return invoke('get_storage_config_cmd')
}

export function saveStorageConfig(args: SaveStorageConfigArgs): Promise<void> {
  return invoke('save_storage_config_cmd', { args })
}

export function clearStorageConfig(): Promise<void> {
  return invoke('clear_storage_config_cmd')
}

export function countMigratableAttachments(newProvider: string): Promise<number> {
  return invoke('count_migratable_attachments_cmd', { newProvider })
}

export function migrateAndSaveStorageConfig(args: SaveStorageConfigArgs): Promise<MigrationSummary> {
  return invoke('migrate_and_save_storage_config_cmd', { newArgs: args })
}

export function listAttachments(assetEventId: number): Promise<AssetAttachment[]> {
  return invoke('list_attachments_cmd', { assetEventId })
}

export function uploadAttachment(assetEventId: number, localFilePath: string): Promise<AssetAttachment> {
  return invoke('upload_attachment_cmd', { assetEventId, localFilePath })
}

export function openAttachment(attachmentId: number): Promise<void> {
  return invoke('open_attachment_cmd', { attachmentId })
}

export function deleteAttachment(attachmentId: number): Promise<void> {
  return invoke('delete_attachment_cmd', { attachmentId })
}

export function listHsaAttachments(hsaExpenseId: number): Promise<HsaAttachment[]> {
  return invoke('list_hsa_attachments_cmd', { hsaExpenseId })
}

export function uploadHsaAttachment(hsaExpenseId: number, localFilePath: string): Promise<HsaAttachment> {
  return invoke('upload_hsa_attachment_cmd', { hsaExpenseId, localFilePath })
}

export function openHsaAttachment(attachmentId: number): Promise<void> {
  return invoke('open_hsa_attachment_cmd', { attachmentId })
}

export function deleteHsaAttachment(attachmentId: number): Promise<void> {
  return invoke('delete_hsa_attachment_cmd', { attachmentId })
}
