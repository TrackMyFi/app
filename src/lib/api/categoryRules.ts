import { invoke } from '@tauri-apps/api/core'
import type { CategoryRule } from '../types/CategoryRule'

export const listCategoryRules = () =>
  invoke<CategoryRule[]>('list_category_rules_cmd')

export const createCategoryRule = (keyword: string, category: string, createdAt: string) =>
  invoke<CategoryRule>('create_category_rule_cmd', { keyword, category, createdAt })

export const deleteCategoryRule = (id: number) =>
  invoke<void>('delete_category_rule_cmd', { id })
