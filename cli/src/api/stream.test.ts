import { describe, it, expect, vi, beforeEach } from 'vitest';

const listeners: Record<string, (event: { data?: string }) => void> = {};
let onerrorHandler: (() => void) | null = null;
const closeMock = vi.fn();

vi.mock('eventsource', () => {
  return {
    EventSource: function MockEventSource() {
      return {
        addEventListener(
          event: string,
          handler: (e: { data?: string }) => void,
        ) {
          listeners[event] = handler;
        },
        close: closeMock,
        set onerror(handler: (() => void) | null) {
          onerrorHandler = handler;
        },
      };
    },
  };
});

import { streamResponse } from './stream.js';
import type { StreamCallbacks } from './stream.js';

describe('streamResponse', () => {
  let callbacks: StreamCallbacks;

  beforeEach(() => {
    vi.clearAllMocks();
    for (const key of Object.keys(listeners)) {
      delete listeners[key];
    }
    onerrorHandler = null;
    callbacks = {
      onToken: vi.fn(),
      onDone: vi.fn(),
      onError: vi.fn(),
    };
  });

  it('calls onToken when token event received', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['token']?.({ data: 'hello' });
    expect(callbacks.onToken).toHaveBeenCalledWith('hello');
  });

  it('calls onDone and closes on done event', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['done']?.({});
    expect(callbacks.onDone).toHaveBeenCalled();
    expect(closeMock).toHaveBeenCalled();
  });

  it('calls onError on error event with data', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['error']?.({ data: 'something broke' });
    expect(callbacks.onError).toHaveBeenCalledWith('something broke');
    expect(closeMock).toHaveBeenCalled();
  });

  it('calls onError on connection error', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    onerrorHandler?.();
    expect(callbacks.onError).toHaveBeenCalledWith('SSE connection failed');
  });

  it('returns cleanup function that closes connection', () => {
    const cleanup = streamResponse(
      'http://localhost:3000',
      'msg-1',
      'tok',
      callbacks,
    );
    cleanup();
    expect(closeMock).toHaveBeenCalled();
  });

  it('accumulates multiple token events', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['token']?.({ data: 'Hello' });
    listeners['token']?.({ data: ' world' });
    expect(callbacks.onToken).toHaveBeenCalledTimes(2);
    expect(callbacks.onToken).toHaveBeenNthCalledWith(1, 'Hello');
    expect(callbacks.onToken).toHaveBeenNthCalledWith(2, ' world');
  });

  it('uses fallback message when error event has no data', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['error']?.({});
    expect(callbacks.onError).toHaveBeenCalledWith('Stream error');
  });

  it('works with different baseUrl and messageId values', () => {
    streamResponse('http://myhost:8080', 'abc-123', 'tok', callbacks);
    listeners['token']?.({ data: 'works' });
    expect(callbacks.onToken).toHaveBeenCalledWith('works');
  });

  it('handles empty string token data', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['token']?.({ data: '' });
    expect(callbacks.onToken).toHaveBeenCalledWith('');
  });

  it('does not call onDone or onError for token events', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['token']?.({ data: 'text' });
    expect(callbacks.onDone).not.toHaveBeenCalled();
    expect(callbacks.onError).not.toHaveBeenCalled();
  });

  it('does not call onToken after done event', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    listeners['done']?.({});
    expect(callbacks.onToken).not.toHaveBeenCalled();
  });

  it('registers all three event listeners', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    expect(listeners['token']).toBeDefined();
    expect(listeners['done']).toBeDefined();
    expect(listeners['error']).toBeDefined();
  });

  it('sets onerror handler', () => {
    streamResponse('http://localhost:3000', 'msg-1', 'tok', callbacks);
    expect(onerrorHandler).toBeDefined();
  });
});
