import React, { useEffect, useState, useCallback } from 'react';
import { Box, Text, useInput } from 'ink';
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
import { ChatView } from './components/ChatView.js';

type AuthState = 'checking' | 'login' | 'authenticated' | 'connection_error';

interface AppProps {
  serverUrl?: string;
}

function isConnectionError(err: unknown): boolean {
  if (err instanceof TypeError && /fetch/i.test(err.message)) return true;
  if (err instanceof Error && /ECONNREFUSED/i.test(err.message)) return true;
  if (
    err instanceof Error &&
    err.cause instanceof Error &&
    /ECONNREFUSED/i.test(err.cause.message)
  )
    return true;
  return false;
}

export function App({ serverUrl }: AppProps) {
  const [authState, setAuthState] = useState<AuthState>('checking');
  const [errorMsg, setErrorMsg] = useState<string>('');

  const checkAuth = useCallback(() => {
    setAuthState('checking');
    setErrorMsg('');

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
      .catch((err: unknown) => {
        if (isConnectionError(err)) {
          const url = serverUrl ?? getServerUrl();
          setErrorMsg(
            `Cannot connect to server at ${url}. Is the backend running?`,
          );
          setAuthState('connection_error');
        } else {
          clearToken();
          setAuthState('login');
        }
      });
  }, [serverUrl]);

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  useInput(
    (input) => {
      if (authState === 'connection_error' && input === 'r') {
        checkAuth();
      }
    },
  );

  if (authState === 'checking') {
    return (
      <Box padding={1}>
        <Text>
          <Spinner type="dots" />{' '}Verifying authentication…
        </Text>
      </Box>
    );
  }

  if (authState === 'connection_error') {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold color="cyan">nexor</Text>
        <Box marginTop={1} flexDirection="column">
          <Text color="red">Connection Error</Text>
          <Text>{errorMsg}</Text>
          <Text dimColor>Press &quot;r&quot; to retry or Ctrl+C to exit.</Text>
        </Box>
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
      <Box marginTop={1} flexDirection="column">
        <ChatView />
      </Box>
    </Box>
  );
}
