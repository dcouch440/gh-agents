import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';

vi.mock('../hooks/useChat.js', () => ({
  useChat: vi.fn().mockReturnValue({
    messages: [],
    loading: true,
    sending: false,
    error: null,
    streamingContent: '',
    isStreaming: false,
    sendMessage: vi.fn(),
  }),
}));

import { ChatView } from './ChatView.js';
import { useChat } from '../hooks/useChat.js';

const mockedUseChat = vi.mocked(useChat);

function mockChat(overrides: Record<string, unknown>) {
  mockedUseChat.mockReturnValue({
    messages: [],
    loading: false,
    sending: false,
    error: null,
    streamingContent: '',
    isStreaming: false,
    sendMessage: vi.fn(),
    ...overrides,
  } as ReturnType<typeof useChat>);
}

describe('ChatView', () => {
  it('shows loading spinner while fetching history', () => {
    mockChat({ loading: true });
    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Loading chat history');
  });

  it('renders messages and input when loaded', () => {
    mockChat({
      messages: [
        {
          id: '1',
          role: 'user',
          content: 'Hello',
          timestamp: '2026-01-29T12:00:00Z',
        },
      ],
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('Hello');
    expect(frame).toContain('>');
  });

  it('shows error message when error is set', () => {
    mockChat({ error: 'Connection refused' });
    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Error: Connection refused');
  });

  it('does not show error when error is null', () => {
    mockChat({});
    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).not.toContain('Error:');
  });

  it('shows sending state while sending', () => {
    mockChat({ sending: true });
    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Sending');
  });

  it('renders multiple messages in order', () => {
    mockChat({
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
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('First');
    expect(frame).toContain('Second');
    expect(frame).toContain('you');
    expect(frame).toContain('nexor');
  });

  it('shows streaming message when streaming', () => {
    mockChat({
      isStreaming: true,
      streamingContent: 'Partial response',
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('Partial response');
  });

  it('shows typing state while streaming', () => {
    mockChat({
      isStreaming: true,
      streamingContent: 'Streaming...',
    });

    const { lastFrame } = render(<ChatView />);
    expect(lastFrame()!).toContain('nexor is typing');
  });

  it('does not show streaming message when not streaming', () => {
    mockChat({
      isStreaming: false,
      streamingContent: '',
    });

    const { lastFrame } = render(<ChatView />);
    // Should not contain the streaming cursor
    expect(lastFrame()!).not.toContain('█');
  });

  it('shows both error and messages', () => {
    mockChat({
      error: 'Temporary failure',
      messages: [
        {
          id: '1',
          role: 'user',
          content: 'My question',
          timestamp: '2026-01-29T12:00:00Z',
        },
      ],
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('Error: Temporary failure');
    expect(frame).toContain('My question');
  });

  it('shows empty state with just input when no messages', () => {
    mockChat({ messages: [] });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('>');
    expect(frame).not.toContain('Error:');
  });

  it('shows streaming content alongside existing messages', () => {
    mockChat({
      messages: [
        {
          id: '1',
          role: 'user',
          content: 'Hello there',
          timestamp: '2026-01-29T12:00:00Z',
        },
      ],
      isStreaming: true,
      streamingContent: 'I am responding...',
    });

    const { lastFrame } = render(<ChatView />);
    const frame = lastFrame()!;
    expect(frame).toContain('Hello there');
    expect(frame).toContain('I am responding...');
  });
});
