import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';

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
    <Box flexDirection="column">
      <Text dimColor>{'─'.repeat(40)}</Text>
      <Box gap={1}>
        <Text bold color="cyan">
          nexor
        </Text>
        <Text dimColor>{time}</Text>
      </Box>
      <Text dimColor>
        {content}
        {!done && cursorVisible ? '█' : ''}
      </Text>
    </Box>
  );
}
