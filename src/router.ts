import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/onboarding', name: 'onboarding', component: () => import('./pages/Onboarding.vue') },
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/accounts/:id', name: 'account-detail', component: () => import('./pages/AccountDetail.vue') },
  { path: '/assets', name: 'assets', component: () => import('./pages/Assets.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
  { path: '/expenses', name: 'expenses', component: () => import('./pages/Expenses.vue') },
  { path: '/paychecks', name: 'paychecks', component: () => import('./pages/Paychecks.vue') },
  { path: '/contributions', name: 'contributions', component: () => import('./pages/Contributions.vue') },
  { path: '/budget', name: 'budget', component: () => import('./pages/Budget.vue') },
  { path: '/forecast', name: 'forecast', component: () => import('./pages/Forecast.vue') },
  { path: '/settings', redirect: '/settings/profile' },
  { path: '/settings/profile', name: 'settings-profile', component: () => import('./pages/settings/FireProfile.vue') },
  { path: '/settings/category-rules', name: 'settings-category-rules', component: () => import('./pages/settings/CategoryRules.vue') },
  { path: '/settings/vendor-rules', name: 'settings-vendor-rules', component: () => import('./pages/settings/VendorRules.vue') },
  { path: '/settings/sync', name: 'settings-sync', component: () => import('./pages/settings/DataSync.vue') },
  { path: '/settings/general', name: 'settings-general', component: () => import('./pages/settings/General.vue') },
]

export const router = createRouter({ history: createWebHashHistory(), routes })
