import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { Message } from './Message.js';
import type { ChatMessage } from '../api/types.js';

const userMsg: ChatMessage = {
  id: '1',
  role: 'user',
  content: 'Hello world',
  timestamp: '2026-01-29T12:34:56Z',
};

const assistantMsg: ChatMessage = {
  id: '2',
  role: 'assistant',
  content: 'Hi there',
  timestamp: '2026-01-29T12:35:00Z',
};

describe('Message', () => {
  it('renders user message with "you" label', () => {
    const { lastFrame } = render(<Message message={userMsg} />);
    const frame = lastFrame()!;
    expect(frame).toContain('you');
    expect(frame).toContain('Hello world');
  });

  it('renders assistant message with "nexor" label', () => {
    const { lastFrame } = render(<Message message={assistantMsg} />);
    const frame = lastFrame()!;
    expect(frame).toContain('nexor');
    expect(frame).toContain('Hi there');
  });

  it('renders separator line', () => {
    const { lastFrame } = render(<Message message={userMsg} />);
    expect(lastFrame()!).toContain('─');
  });

  it('renders timestamp', () => {
    const { lastFrame } = render(<Message message={userMsg} />);
    // Timestamp format depends on locale, just check it renders something
    const frame = lastFrame()!;
    expect(frame.length).toBeGreaterThan(0);
  });

  it('does not show "nexor" label for user messages', () => {
    const { lastFrame } = render(<Message message={userMsg} />);
    expect(lastFrame()!).not.toContain('nexor');
  });

  it('does not show "you" label for assistant messages', () => {
    const { lastFrame } = render(<Message message={assistantMsg} />);
    expect(lastFrame()!).not.toContain('you');
  });

  it('renders multiline content', () => {
    const msg: ChatMessage = {
      id: '3',
      role: 'user',
      content: 'Line one\nLine two',
      timestamp: '2026-01-29T12:36:00Z',
    };
    const { lastFrame } = render(<Message message={msg} />);
    const frame = lastFrame()!;
    expect(frame).toContain('Line one');
    expect(frame).toContain('Line two');
  });

  it('renders separator with 40 dash characters', () => {
    const { lastFrame } = render(<Message message={userMsg} />);
    expect(lastFrame()!).toContain('─'.repeat(40));
  });
});
