import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { MessageList } from './MessageList.js';
import { StreamingMessage } from './StreamingMessage.js';
import { Input } from './Input.js';
import { useChat } from '../hooks/useChat.js';

export function ChatView() {
  const {
    messages,
    loading,
    sending,
    error,
    streamingContent,
    isStreaming,
    sendMessage,
  } = useChat();

  if (loading) {
    return (
      <Box padding={1}>
        <Text>
          <Spinner type="dots" /> Loading chat history…
        </Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      {error && (
        <Box>
          <Text color="red">Error: {error}</Text>
        </Box>
      )}
      <MessageList messages={messages} />
      {isStreaming && (
        <StreamingMessage content={streamingContent} done={false} />
      )}
      <Box marginTop={1}>
        <Input onSubmit={sendMessage} disabled={sending || isStreaming} />
      </Box>
    </Box>
  );
}
