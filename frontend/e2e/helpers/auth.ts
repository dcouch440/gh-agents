import type { Page, APIRequestContext } from '@playwright/test'

const API_BASE = 'http://localhost:5173/api'
const TEST_EMAIL = 'e2e-test@nexor.dev'
const TEST_PASSWORD = 'e2e-test-password-123'

/**
 * Register or login a test user via the real API and return the JWT token.
 * Tries login first; if that fails (user doesn't exist), registers instead.
 */
export const getAuthToken = async (request: APIRequestContext): Promise<string> => {
  // Try login first
  const loginRes = await request.post(`${API_BASE}/auth/login`, {
    data: { email: TEST_EMAIL, password: TEST_PASSWORD },
  })

  if (loginRes.ok()) {
    const body = await loginRes.json() as { token: string }
    return body.token
  }

  // Login failed — register
  const registerRes = await request.post(`${API_BASE}/auth/register`, {
    data: { email: TEST_EMAIL, password: TEST_PASSWORD },
  })

  if (!registerRes.ok()) {
    const text = await registerRes.text()
    throw new Error(`Auth setup failed: ${registerRes.status()} ${text}`)
  }

  const body = await registerRes.json() as { token: string }
  return body.token
}

/**
 * Inject the auth token into localStorage before the page loads,
 * so the app hydrates as an authenticated user.
 */
export const setupAuth = async (page: Page, token: string): Promise<void> => {
  await page.addInitScript((t) => {
    localStorage.setItem('nexor_auth_token', t)
  }, token)
}
