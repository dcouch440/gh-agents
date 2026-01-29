import React from 'react';
import { Box, Text } from 'ink';
import type { ChatMessage } from '../api/types.js';

interface MessageProps {
  message: ChatMessage;
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString();
}

export function Message({ message }: MessageProps) {
  const isUser = message.role === 'user';
  const label = isUser ? 'you' : 'nexor';

  return (
    <Box flexDirection="column">
      <Text dimColor>{'─'.repeat(40)}</Text>
      <Box gap={1}>
        <Text bold color={isUser ? undefined : 'cyan'}>
          {label}
        </Text>
        <Text dimColor>{formatTime(message.timestamp)}</Text>
      </Box>
      <Text dimColor={!isUser}>{message.content}</Text>
    </Box>
  );
}
