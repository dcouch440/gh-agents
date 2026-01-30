import { api } from '../api/client.js';
import { setToken } from '../store/auth.js';

export interface LoginResult {
  success: boolean;
  error?: string;
}

export async function handleLogin(password: string): Promise<LoginResult> {
  if (!password) {
    return { success: false, error: 'Password is required.' };
  }
  try {
    const res = await api.auth.login(password);
    setToken(res.token, res.expires_in);
    return { success: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { success: false, error: `Authentication failed: ${msg}` };
  }
}
