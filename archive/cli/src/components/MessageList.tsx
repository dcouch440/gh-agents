import React from 'react';
import { Box } from 'ink';
import { Message } from './Message.js';
import type { ChatMessage } from '../api/types.js';

interface MessageListProps {
  messages: ChatMessage[];
}

export function MessageList({ messages }: MessageListProps) {
  return (
    <Box flexDirection="column">
      {messages.map((msg) => (
        <Message key={msg.id} message={msg} />
      ))}
    </Box>
  );
}
