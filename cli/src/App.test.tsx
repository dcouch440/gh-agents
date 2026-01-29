import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { App } from './App.js';

vi.mock('./api/client.js', () => ({
  api: {
    auth: {
      me: vi.fn(),
    },
  },
  setBaseUrl: vi.fn(),
  setToken: vi.fn(),
}));

vi.mock('./store/auth.js', () => ({
  getToken: vi.fn(),
  isTokenExpired: vi.fn(),
  clearToken: vi.fn(),
  getServerUrl: vi.fn().mockReturnValue('http://localhost:3000'),
  setServerUrl: vi.fn(),
}));

vi.mock('./components/Login.js', () => ({
  Login: ({ onSuccess }: { onSuccess: () => void }) => {
    return React.createElement('ink-text', null, 'MockLogin');
  },
}));

vi.mock('./components/ChatView.js', () => ({
  ChatView: () => React.createElement('ink-text', null, 'MockChatView'),
}));

import { api, setBaseUrl, setToken as setApiToken } from './api/client.js';
import {
  getToken,
  isTokenExpired,
  clearToken,
  getServerUrl,
  setServerUrl,
} from './store/auth.js';

const mockedMe = vi.mocked(api.auth.me);
const mockedGetToken = vi.mocked(getToken);
const mockedIsTokenExpired = vi.mocked(isTokenExpired);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(getServerUrl).mockReturnValue('http://localhost:3000');
});

describe('App', () => {
  it('shows login when no token is stored', async () => {
    mockedGetToken.mockReturnValue(null);
    mockedIsTokenExpired.mockReturnValue(true);

    const { lastFrame } = render(<App />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('MockLogin');
    });
    expect(clearToken).toHaveBeenCalled();
  });

  it('shows login when token is expired', async () => {
    mockedGetToken.mockReturnValue('expired-token');
    mockedIsTokenExpired.mockReturnValue(true);

    const { lastFrame } = render(<App />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('MockLogin');
    });
    expect(clearToken).toHaveBeenCalled();
  });

  it('verifies token and shows authenticated on success', async () => {
    mockedGetToken.mockReturnValue('valid-token');
    mockedIsTokenExpired.mockReturnValue(false);
    mockedMe.mockResolvedValue({
      user: 'admin',
      authenticated: true,
      token_expires: 9999,
    });

    const { lastFrame } = render(<App />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('MockChatView');
    });
    expect(setApiToken).toHaveBeenCalledWith('valid-token');
  });

  it('falls back to login when me() rejects', async () => {
    mockedGetToken.mockReturnValue('bad-token');
    mockedIsTokenExpired.mockReturnValue(false);
    mockedMe.mockRejectedValue(new Error('401'));

    const { lastFrame } = render(<App />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('MockLogin');
    });
  });

  it('sets custom server URL from prop', async () => {
    mockedGetToken.mockReturnValue(null);
    mockedIsTokenExpired.mockReturnValue(true);

    render(<App serverUrl="http://custom:8080" />);

    await vi.waitFor(() => {
      expect(setServerUrl).toHaveBeenCalledWith('http://custom:8080');
      expect(setBaseUrl).toHaveBeenCalledWith('http://custom:8080');
    });
  });

  it('uses stored server URL when no prop provided', async () => {
    mockedGetToken.mockReturnValue(null);
    mockedIsTokenExpired.mockReturnValue(true);

    render(<App />);

    await vi.waitFor(() => {
      expect(setBaseUrl).toHaveBeenCalledWith('http://localhost:3000');
    });
  });

  it('transitions to authenticated when Login onSuccess fires', async () => {
    // First render: no token → login state
    mockedGetToken.mockReturnValue(null);
    mockedIsTokenExpired.mockReturnValue(true);

    // Re-mock Login to call onSuccess immediately
    const { Login: _orig, ...rest } = await import('./components/Login.js');
    vi.mocked(await import('./components/Login.js')).Login;
    // We need to override the mock to trigger onSuccess
    vi.doMock('./components/Login.js', () => ({
      Login: ({ onSuccess }: { onSuccess: () => void }) => {
        // Simulate calling onSuccess on mount
        React.useEffect(() => { onSuccess(); }, []);
        return React.createElement('ink-text', null, 'MockLogin');
      },
    }));

    // Re-import App with new mock
    vi.resetModules();
    // Re-setup auth mock
    vi.doMock('./store/auth.js', () => ({
      getToken: vi.fn().mockReturnValueOnce(null).mockReturnValue('new-token'),
      isTokenExpired: vi.fn().mockReturnValue(true),
      clearToken: vi.fn(),
      getServerUrl: vi.fn().mockReturnValue('http://localhost:3000'),
      setServerUrl: vi.fn(),
    }));
    vi.doMock('./api/client.js', () => ({
      api: { auth: { me: vi.fn() } },
      setBaseUrl: vi.fn(),
      setToken: vi.fn(),
    }));
    vi.doMock('./components/ChatView.js', () => ({
      ChatView: () => React.createElement('ink-text', null, 'MockChatView'),
    }));

    const { App: FreshApp } = await import('./App.js');
    const { setToken: freshSetApiToken } = await import('./api/client.js');

    const { lastFrame } = render(React.createElement(FreshApp));

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('MockChatView');
    });
    expect(freshSetApiToken).toHaveBeenCalledWith('new-token');
  });
});
