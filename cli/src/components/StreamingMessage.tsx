import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { Markdown } from './Markdown.js';

interface StreamingMessageProps {
  content: string;
  done: boolean;
}

export function StreamingMessage({ content, done }: StreamingMessageProps) {
  const [cursorVisible, setCursorVisible] = useState(true);

  useEffect(() => {
    if (done) return;
    const interval = setInterval(() => {
      setCursorVisible((v: boolean) => !v);
    }, 500);
    return () => clearInterval(interval);
  }, [done]);

  const time = new Date().toLocaleTimeString();

  return (
    <Box flexDirection="column" marginTop={1}>
      <Text dimColor>{'─'.repeat(60)}</Text>
      <Box gap={1}>
        <Text bold color="cyan">
          nexor
        </Text>
        <Text dimColor>{time}</Text>
      </Box>
      <Box paddingLeft={2}>
        {done ? (
          <Markdown content={content} />
        ) : (
          <Text>
            {content}
            {cursorVisible ? '█' : ''}
          </Text>
        )}
      </Box>
    </Box>
  );
}
