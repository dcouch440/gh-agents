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
  } catch {
    return { success: false, error: 'Authentication failed. Please try again.' };
  }
}
