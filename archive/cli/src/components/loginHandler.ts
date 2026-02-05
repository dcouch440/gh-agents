import { api } from '../api/client.js';
import { setToken } from '../store/auth.js';

export interface LoginResult {
  success: boolean;
  error?: string;
}

export async function handleLogin(email: string, password: string): Promise<LoginResult> {
  if (!email || !password) {
    return { success: false, error: 'Email and password are required.' };
  }
  try {
    const res = await api.auth.login(email, password);
    setToken(res.token, res.expires_in);
    return { success: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { success: false, error: `Authentication failed: ${msg}` };
  }
}
