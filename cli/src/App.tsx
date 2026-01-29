import React, { useEffect, useState } from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { api, setBaseUrl, setToken as setApiToken } from './api/client.js';
import {
  getToken,
  isTokenExpired,
  clearToken,
  getServerUrl,
  setServerUrl,
} from './store/auth.js';
import { Login } from './components/Login.js';

type AuthState = 'checking' | 'login' | 'authenticated';

interface AppProps {
  serverUrl?: string;
}

export function App({ serverUrl }: AppProps) {
  const [authState, setAuthState] = useState<AuthState>('checking');

  useEffect(() => {
    if (serverUrl) {
      setServerUrl(serverUrl);
      setBaseUrl(serverUrl);
    } else {
      setBaseUrl(getServerUrl());
    }

    const token = getToken();
    if (!token || isTokenExpired()) {
      clearToken();
      setAuthState('login');
      return;
    }

    setApiToken(token);
    api.auth
      .me()
      .then(() => setAuthState('authenticated'))
      .catch(() => {
        clearToken();
        setAuthState('login');
      });
  }, [serverUrl]);

  if (authState === 'checking') {
    return (
      <Box padding={1}>
        <Text>
          <Spinner type="dots" />{' '}Verifying authentication…
        </Text>
      </Box>
    );
  }

  if (authState === 'login') {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold color="cyan">nexor</Text>
        <Text dimColor>AI agent orchestration for GitHub workflows</Text>
        <Box marginTop={1}>
          <Login
            onSuccess={() => {
              const token = getToken();
              if (token) setApiToken(token);
              setAuthState('authenticated');
            }}
          />
        </Box>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" padding={1}>
      <Text bold color="cyan">nexor</Text>
      <Text dimColor>AI agent orchestration for GitHub workflows</Text>
      <Box marginTop={1}>
        <Text color="green">✓ Authenticated</Text>
      </Box>
    </Box>
  );
}
