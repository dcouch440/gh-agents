import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';

interface InputProps {
  onSubmit: (value: string) => void;
  disabled?: boolean;
}

export function Input({ onSubmit, disabled }: InputProps) {
  const [value, setValue] = useState('');

  const handleSubmit = (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
    setValue('');
  };

  if (disabled) {
    return (
      <Box>
        <Text dimColor>Waiting for response…</Text>
      </Box>
    );
  }

  return (
    <Box>
      <Text bold color="green">{'> '}</Text>
      <TextInput value={value} onChange={setValue} onSubmit={handleSubmit} />
    </Box>
  );
}
