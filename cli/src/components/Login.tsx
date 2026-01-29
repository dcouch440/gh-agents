import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { handleLogin } from './loginHandler.js';

interface LoginProps {
  onSuccess: () => void;
}

export function Login({ onSuccess }: LoginProps) {
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (value: string) => {
    if (!value) return;
    setLoading(true);
    setError(null);
    const result = await handleLogin(value);
    if (result.success) {
      onSuccess();
    } else {
      setError(result.error ?? 'Unknown error');
      setPassword('');
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <Box>
        <Text>
          <Spinner type="dots" />{' '}Authenticating…
        </Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      <Box>
        <Text>Password: </Text>
        <TextInput
          value={password}
          onChange={setPassword}
          onSubmit={handleSubmit}
          mask="*"
        />
      </Box>
      {error && <Text color="red">{error}</Text>}
    </Box>
  );
}
