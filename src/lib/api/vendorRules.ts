import { invoke } from '@tauri-apps/api/core'
import type { VendorRule } from '../types/VendorRule'

export const listVendorRules = () =>
  invoke<VendorRule[]>('list_vendor_rules_cmd')

export const createVendorRule = (keyword: string, vendorName: string, createdAt: string) =>
  invoke<VendorRule>('create_vendor_rule_cmd', { keyword, vendorName, createdAt })

export const deleteVendorRule = (id: number) =>
  invoke<void>('delete_vendor_rule_cmd', { id })
