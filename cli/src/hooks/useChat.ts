import { useState, useEffect, useCallback, useRef } from 'react';
import { api, getBaseUrl, getToken } from '../api/client.js';
import { streamResponse } from '../api/stream.js';
import type { ChatMessage } from '../api/types.js';

interface UseChatResult {
  messages: ChatMessage[];
  loading: boolean;
  sending: boolean;
  error: string | null;
  streamingContent: string;
  isStreaming: boolean;
  sendMessage: (content: string) => Promise<void>;
}

const FLUSH_INTERVAL_MS = 50;

export function useChat(): UseChatResult {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [streamingContent, setStreamingContent] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const cleanupRef = useRef<(() => void) | null>(null);
  const tokenBufferRef = useRef('');
  const flushIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    let cancelled = false;
    api.chat
      .history()
      .then((history) => {
        if (!cancelled) {
          setMessages(history);
          setLoading(false);
        }
      })
      .catch((err: Error) => {
        if (!cancelled) {
          setError(err.message);
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    return () => {
      cleanupRef.current?.();
      if (flushIntervalRef.current) {
        clearInterval(flushIntervalRef.current);
      }
    };
  }, []);

  const sendMessage = useCallback(async (content: string) => {
    const optimistic: ChatMessage = {
      id: `temp-${Date.now()}`,
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev: ChatMessage[]) => [...prev, optimistic]);
    setSending(true);
    setError(null);

    try {
      const response = await api.chat.send(content);
      setSending(false);
      setIsStreaming(true);
      setStreamingContent('');

      // Start flush interval for batched token updates
      flushIntervalRef.current = setInterval(() => {
        if (tokenBufferRef.current) {
          const buffered = tokenBufferRef.current;
          tokenBufferRef.current = '';
          setStreamingContent((prev: string) => prev + buffered);
        }
      }, FLUSH_INTERVAL_MS);

      const cleanup = streamResponse(
        getBaseUrl(),
        response.message_id,
        getToken(),
        {
          onToken: (text: string) => {
            tokenBufferRef.current += text;
          },
          onDone: () => {
            // Flush remaining tokens
            if (tokenBufferRef.current) {
              const remaining = tokenBufferRef.current;
              tokenBufferRef.current = '';
              setStreamingContent((prev: string) => prev + remaining);
            }
            if (flushIntervalRef.current) {
              clearInterval(flushIntervalRef.current);
              flushIntervalRef.current = null;
            }
            setIsStreaming(false);
            cleanupRef.current = null;
            // Reload history to get the full message with real IDs
            api.chat
              .history()
              .then((history) => setMessages(history))
              .catch(() => {});
          },
          onError: (errMsg: string) => {
            tokenBufferRef.current = '';
            if (flushIntervalRef.current) {
              clearInterval(flushIntervalRef.current);
              flushIntervalRef.current = null;
            }
            setIsStreaming(false);
            setStreamingContent('');
            setError(errMsg);
            cleanupRef.current = null;
          },
        },
      );
      cleanupRef.current = cleanup;
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Send failed';
      setError(msg);
      setSending(false);
    }
  }, []);

  return {
    messages,
    loading,
    sending,
    error,
    streamingContent,
    isStreaming,
    sendMessage,
  };
}
