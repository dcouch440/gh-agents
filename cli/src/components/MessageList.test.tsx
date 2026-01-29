import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { MessageList } from './MessageList.js';
import type { ChatMessage } from '../api/types.js';

const messages: ChatMessage[] = [
  { id: '1', role: 'user', content: 'First message', timestamp: '2026-01-29T12:00:00Z' },
  { id: '2', role: 'assistant', content: 'Second message', timestamp: '2026-01-29T12:00:01Z' },
  { id: '3', role: 'user', content: 'Third message', timestamp: '2026-01-29T12:00:02Z' },
];

describe('MessageList', () => {
  it('renders all messages', () => {
    const { lastFrame } = render(<MessageList messages={messages} />);
    const frame = lastFrame()!;
    expect(frame).toContain('First message');
    expect(frame).toContain('Second message');
    expect(frame).toContain('Third message');
  });

  it('renders empty list without error', () => {
    const { lastFrame } = render(<MessageList messages={[]} />);
    expect(lastFrame()).toBeDefined();
  });

  it('renders role labels for each message', () => {
    const { lastFrame } = render(<MessageList messages={messages} />);
    const frame = lastFrame()!;
    expect(frame).toContain('you');
    expect(frame).toContain('nexor');
  });

  it('renders a single message', () => {
    const single = [messages[0]];
    const { lastFrame } = render(<MessageList messages={single} />);
    expect(lastFrame()!).toContain('First message');
  });

  it('renders separators between messages', () => {
    const { lastFrame } = render(<MessageList messages={messages} />);
    const frame = lastFrame()!;
    const separatorCount = (frame.match(/─{40}/g) || []).length;
    expect(separatorCount).toBe(3);
  });
});
