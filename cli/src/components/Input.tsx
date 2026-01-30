import React, { useState } from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import TextInput from 'ink-text-input';

interface InputProps {
  onSubmit: (value: string) => void;
  sending?: boolean;
  isStreaming?: boolean;
}

export function Input({ onSubmit, sending, isStreaming }: InputProps) {
  const [value, setValue] = useState('');

  const handleSubmit = (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
    setValue('');
  };

  if (sending) {
    return (
      <Box>
        <Text>
          <Spinner type="dots" /> Sending…
        </Text>
      </Box>
    );
  }

  if (isStreaming) {
    return (
      <Box>
        <Text>
          <Spinner type="dots" /> nexor is typing…
        </Text>
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
