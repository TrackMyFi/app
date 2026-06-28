import { invoke } from '@tauri-apps/api/core'
import type { AssetAttachment } from '../types/AssetAttachment'
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
