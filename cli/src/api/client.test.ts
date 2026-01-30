import { describe, it, expect, beforeEach, vi } from 'vitest';
import { api, setBaseUrl, setToken, getBaseUrl, getToken } from './client.js';

const TEST_BASE = 'http://localhost:9999';

function mockFetch(status: number, body: unknown, ok?: boolean) {
  return vi.fn().mockResolvedValue({
    ok: ok ?? (status >= 200 && status < 300),
    status,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(JSON.stringify(body)),
  });
}

beforeEach(() => {
  setBaseUrl(TEST_BASE);
  setToken('');
  vi.restoreAllMocks();
});

describe('setBaseUrl', () => {
  it('strips trailing slashes', () => {
    setBaseUrl('http://example.com///');
    const spy = mockFetch(200, { status: 'ok' });
    vi.stubGlobal('fetch', spy);
    api.health();
    expect(spy).toHaveBeenCalledWith(
      'http://example.com/health',
      expect.anything(),
    );
  });
});

describe('setToken', () => {
  it('adds Authorization header when token is set', async () => {
    const spy = mockFetch(200, { status: 'ok' });
    vi.stubGlobal('fetch', spy);
    setToken('my-token');
    await api.health();
    const headers = spy.mock.calls[0][1].headers;
    expect(headers['Authorization']).toBe('Bearer my-token');
  });

  it('does not add Authorization header when token is empty', async () => {
    const spy = mockFetch(200, { status: 'ok' });
    vi.stubGlobal('fetch', spy);
    await api.health();
    const headers = spy.mock.calls[0][1].headers;
    expect(headers['Authorization']).toBeUndefined();
  });
});

describe('error handling', () => {
  it('throws ApiError with status and body on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch(401, { error: 'unauthorized' }, false));
    await expect(api.health()).rejects.toThrow('API error 401');
  });

  it('exposes status and body properties on ApiError', async () => {
    const errorBody = { error: 'forbidden' };
    vi.stubGlobal('fetch', mockFetch(403, errorBody, false));
    try {
      await api.health();
      expect.fail('should have thrown');
    } catch (err: unknown) {
      const e = err as { name: string; status: number; body: unknown };
      expect(e.name).toBe('ApiError');
      expect(e.status).toBe(403);
      expect(e.body).toEqual(errorBody);
    }
  });

  it('falls back to text body when json parsing fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        json: () => Promise.reject(new Error('bad json')),
        text: () => Promise.resolve('Internal Server Error'),
      }),
    );
    try {
      await api.health();
      expect.fail('should have thrown');
    } catch (err: unknown) {
      const e = err as { status: number; body: unknown };
      expect(e.status).toBe(500);
      expect(e.body).toBe('Internal Server Error');
    }
  });

  it('propagates network errors when fetch rejects', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockRejectedValue(new TypeError('fetch failed')),
    );
    await expect(api.health()).rejects.toThrow('fetch failed');
  });

  it('always sets Content-Type to application/json', async () => {
    const spy = mockFetch(200, {});
    vi.stubGlobal('fetch', spy);
    await api.health();
    expect(spy.mock.calls[0][1].headers['Content-Type']).toBe(
      'application/json',
    );
  });
});

describe('204 No Content', () => {
  it('returns undefined for 204 responses', async () => {
    vi.stubGlobal('fetch', mockFetch(204, undefined));
    const result = await api.chat.clear();
    expect(result).toBeUndefined();
  });
});

describe('api.health', () => {
  it('calls GET /health', async () => {
    const body = { status: 'ok', version: '0.1.0', db_connected: true };
    const spy = mockFetch(200, body);
    vi.stubGlobal('fetch', spy);
    const result = await api.health();
    expect(spy).toHaveBeenCalledWith(`${TEST_BASE}/health`, expect.anything());
    expect(result).toEqual(body);
  });
});

describe('api.auth', () => {
  it('login sends POST /auth/login with password', async () => {
    const body = { token: 'jwt', expires_in: 3600 };
    const spy = mockFetch(200, body);
    vi.stubGlobal('fetch', spy);
    const result = await api.auth.login('test@test.com', 'secret');
    expect(spy).toHaveBeenCalledWith(
      `${TEST_BASE}/auth/login`,
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ email: 'test@test.com', password: 'secret' }),
      }),
    );
    expect(result).toEqual(body);
  });

  it('me sends GET /auth/me', async () => {
    const body = { user: 'admin', authenticated: true, token_expires: 9999 };
    const spy = mockFetch(200, body);
    vi.stubGlobal('fetch', spy);
    const result = await api.auth.me();
    expect(spy).toHaveBeenCalledWith(`${TEST_BASE}/auth/me`, expect.anything());
    expect(result).toEqual(body);
  });
});

describe('getBaseUrl', () => {
  it('returns the current base URL', () => {
    setBaseUrl('http://example.com:4000');
    expect(getBaseUrl()).toBe('http://example.com:4000');
  });

  it('reflects trailing slash stripping', () => {
    setBaseUrl('http://example.com:4000///');
    expect(getBaseUrl()).toBe('http://example.com:4000');
  });
});

describe('getToken', () => {
  it('returns empty string when no token is set', () => {
    setToken('');
    expect(getToken()).toBe('');
  });

  it('returns the current token', () => {
    setToken('my-secret-token');
    expect(getToken()).toBe('my-secret-token');
  });
});

describe('api.chat', () => {
  it('send sends POST /chat with message', async () => {
    const body = { message_id: '1', status: 'ok' };
    const spy = mockFetch(200, body);
    vi.stubGlobal('fetch', spy);
    const result = await api.chat.send('hello');
    expect(spy).toHaveBeenCalledWith(
      `${TEST_BASE}/chat`,
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ message: 'hello' }),
      }),
    );
    expect(result).toEqual(body);
  });

  it('history sends GET /chat/history with default params', async () => {
    const spy = mockFetch(200, []);
    vi.stubGlobal('fetch', spy);
    await api.chat.history();
    expect(spy).toHaveBeenCalledWith(
      `${TEST_BASE}/chat/history?limit=50&offset=0`,
      expect.anything(),
    );
  });

  it('history sends GET /chat/history with custom params', async () => {
    const spy = mockFetch(200, []);
    vi.stubGlobal('fetch', spy);
    await api.chat.history(10, 5);
    expect(spy).toHaveBeenCalledWith(
      `${TEST_BASE}/chat/history?limit=10&offset=5`,
      expect.anything(),
    );
  });

  it('clear sends DELETE /chat/history', async () => {
    const spy = mockFetch(204, undefined);
    vi.stubGlobal('fetch', spy);
    await api.chat.clear();
    expect(spy).toHaveBeenCalledWith(
      `${TEST_BASE}/chat/history`,
      expect.objectContaining({ method: 'DELETE' }),
    );
  });
});
