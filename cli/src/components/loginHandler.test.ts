import { describe, it, expect, vi, beforeEach } from 'vitest';
import { handleLogin } from './loginHandler.js';

vi.mock('../api/client.js', () => ({
  api: {
    auth: {
      login: vi.fn(),
    },
  },
}));

vi.mock('../store/auth.js', () => ({
  setToken: vi.fn(),
}));

import { api } from '../api/client.js';
import { setToken } from '../store/auth.js';

const mockedLogin = vi.mocked(api.auth.login);
const mockedSetToken = vi.mocked(setToken);

beforeEach(() => {
  vi.clearAllMocks();
});

describe('handleLogin', () => {
  it('returns error when password is empty', async () => {
    const result = await handleLogin('');
    expect(result.success).toBe(false);
    expect(result.error).toBe('Password is required.');
    expect(mockedLogin).not.toHaveBeenCalled();
  });

  it('calls api.auth.login with password and stores token on success', async () => {
    mockedLogin.mockResolvedValue({ token: 'jwt-abc', expires_in: 3600 });

    const result = await handleLogin('mysecret');

    expect(result.success).toBe(true);
    expect(result.error).toBeUndefined();
    expect(mockedLogin).toHaveBeenCalledWith('mysecret');
    expect(mockedSetToken).toHaveBeenCalledWith('jwt-abc', 3600);
  });

  it('returns error on authentication failure', async () => {
    mockedLogin.mockRejectedValue(new Error('401 Unauthorized'));

    const result = await handleLogin('wrong');

    expect(result.success).toBe(false);
    expect(result.error).toBe('Authentication failed. Please try again.');
    expect(mockedSetToken).not.toHaveBeenCalled();
  });

  it('returns error on network failure', async () => {
    mockedLogin.mockRejectedValue(new TypeError('fetch failed'));

    const result = await handleLogin('password');

    expect(result.success).toBe(false);
    expect(result.error).toContain('Authentication failed');
  });
});
