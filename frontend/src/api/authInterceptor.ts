import { addInterceptor } from './client'
import { hasStatus } from './guards'
import { API } from '@/constants'
import { authStore } from '@/stores/authStore'

// ── Auth endpoints excluded from 401 auto-logout ────────────────────────────

const AUTH_ENDPOINTS = new Set([API.AUTH_LOGIN, API.AUTH_REGISTER])

const isAuthEndpoint = (url: string): boolean => {
  for (const endpoint of AUTH_ENDPOINTS) {
    if (url.endsWith(endpoint)) return true
  }
  return false
}

// ── Setup ───────────────────────────────────────────────────────────────────

const setupAuthInterceptor = (): (() => void) =>
  addInterceptor({
    onError: (error) => {
      if (hasStatus(error, 401) && !isAuthEndpoint(error.url)) {
        authStore.logout()
      }
      return error
    },
  })

export { setupAuthInterceptor }
