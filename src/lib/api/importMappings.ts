import { invoke } from '@tauri-apps/api/core'
import type { ImportMapping } from '../types/ImportMapping'

export interface NewImportMapping {
  name: string
  config: string
  createdAt: string
}

export const listImportMappings = () =>
  invoke<ImportMapping[]>('list_import_mappings_cmd')
export const createImportMapping = (mapping: NewImportMapping) =>
  invoke<number>('create_import_mapping_cmd', { mapping })
export const deleteImportMapping = (id: number) =>
  invoke<void>('delete_import_mapping_cmd', { id })
