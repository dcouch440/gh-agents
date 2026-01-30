import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { renderMarkdown } from '../utils/markdown.js';

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
  const isThinking = !content && !done;

  return (
    <Box flexDirection="column">
      <Text dimColor>{'─'.repeat(40)}</Text>
      <Box gap={1}>
        <Text bold color="cyan">
          nexor
        </Text>
        <Text dimColor>{time}</Text>
      </Box>
      {isThinking ? (
        <Text>
          <Spinner type="dots" /> Thinking…
        </Text>
      ) : (
        <Text>
          {content ? renderMarkdown(content) : ''}
          {!done && cursorVisible ? '█' : ''}
        </Text>
      )}
    </Box>
  );
}
