import React from 'react';
import { Box, Text } from 'ink';
import type { ChatMessage } from '../api/types.js';
import { Markdown } from './Markdown.js';

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
    <Box flexDirection="column" marginTop={1}>
      <Text dimColor>{'─'.repeat(60)}</Text>
      <Box gap={1}>
        <Text bold color={isUser ? 'green' : 'cyan'}>
          {label}
        </Text>
        <Text dimColor>{formatTime(message.timestamp)}</Text>
      </Box>
      <Box paddingLeft={2}>
        {isUser ? <Text>{message.content}</Text> : <Markdown content={message.content} />}
      </Box>
    </Box>
  );
}
