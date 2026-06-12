import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
  { path: '/paychecks', name: 'paychecks', component: () => import('./pages/Paychecks.vue') },
  { path: '/settings', name: 'settings', component: () => import('./pages/Settings.vue') },
]

export const router = createRouter({ history: createWebHashHistory(), routes })
