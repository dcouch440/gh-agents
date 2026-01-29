import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';

// Default mock: loading state (history never resolves)
vi.mock('../hooks/useChat.js', () => ({
  useChat: vi.fn().mockReturnValue({
    messages: [],
    loading: true,
    sending: false,
    error: null,
    sendMessage: vi.fn(),
  }),
}));

import { ChatView } from './ChatView.js';
import { useChat } from '../hooks/useChat.js';

const mockedUseChat = vi.mocked(useChat);

describe('ChatView', () => {
  it('shows loading spinner while fetching history', () => {
    mockedUseChat.mockReturnValue({
      messages: [],
      loading: true,
      sending: false,
      error: null,
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Loading chat history');
  });

  it('renders messages and input when loaded', () => {
    mockedUseChat.mockReturnValue({
      messages: [
        {
          id: '1',
          role: 'user',
          content: 'Hello',
          timestamp: '2026-01-29T12:00:00Z',
        },
      ],
      loading: false,
      sending: false,
      error: null,
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('Hello');
    expect(frame).toContain('>');
  });

  it('shows error message when error is set', () => {
    mockedUseChat.mockReturnValue({
      messages: [],
      loading: false,
      sending: false,
      error: 'Connection refused',
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Error: Connection refused');
  });

  it('does not show error when error is null', () => {
    mockedUseChat.mockReturnValue({
      messages: [],
      loading: false,
      sending: false,
      error: null,
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).not.toContain('Error:');
  });

  it('disables input while sending', () => {
    mockedUseChat.mockReturnValue({
      messages: [],
      loading: false,
      sending: true,
      error: null,
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Waiting for response');
  });

  it('renders multiple messages in order', () => {
    mockedUseChat.mockReturnValue({
      messages: [
        {
          id: '1',
          role: 'user',
          content: 'First',
          timestamp: '2026-01-29T12:00:00Z',
        },
        {
          id: '2',
          role: 'assistant',
          content: 'Second',
          timestamp: '2026-01-29T12:00:01Z',
        },
      ],
      loading: false,
      sending: false,
      error: null,
      sendMessage: vi.fn(),
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('First');
    expect(frame).toContain('Second');
    expect(frame).toContain('you');
    expect(frame).toContain('nexor');
  });
});
