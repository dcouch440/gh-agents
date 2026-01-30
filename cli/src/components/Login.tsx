import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { handleLogin } from './loginHandler.js';

interface LoginProps {
  onSuccess: () => void;
}

export function Login({ onSuccess }: LoginProps) {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [step, setStep] = useState<'email' | 'password'>('email');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleEmailSubmit = (value: string) => {
    if (!value) return;
    setStep('password');
  };

  const handleSubmit = async (value: string) => {
    if (!value) return;
    setLoading(true);
    setError(null);
    const result = await handleLogin(email, value);
    if (result.success) {
      onSuccess();
    } else {
      setError(result.error ?? 'Unknown error');
      setPassword('');
      setStep('email');
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
      {step === 'email' ? (
        <Box>
          <Text>Email: </Text>
          <TextInput
            value={email}
            onChange={setEmail}
            onSubmit={handleEmailSubmit}
          />
        </Box>
      ) : (
        <Box>
          <Text>Password: </Text>
          <TextInput
            value={password}
            onChange={setPassword}
            onSubmit={handleSubmit}
            mask="*"
          />
        </Box>
      )}
      {error && <Text color="red">{error}</Text>}
    </Box>
  );
}
