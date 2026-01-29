import { useState, useEffect, useCallback } from 'react';
import { api } from '../api/client.js';
import type { ChatMessage } from '../api/types.js';

interface UseChatResult {
  messages: ChatMessage[];
  loading: boolean;
  sending: boolean;
  error: string | null;
  sendMessage: (content: string) => Promise<void>;
}

export function useChat(): UseChatResult {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  const sendMessage = useCallback(async (content: string) => {
    const optimistic: ChatMessage = {
      id: `temp-${Date.now()}`,
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, optimistic]);
    setSending(true);
    setError(null);

    try {
      await api.chat.send(content);
      // Reload history to get the assistant response and real IDs
      const history = await api.chat.history();
      setMessages(history);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Send failed';
      setError(msg);
    } finally {
      setSending(false);
    }
  }, []);

  return { messages, loading, sending, error, sendMessage };
}
