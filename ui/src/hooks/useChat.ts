import { useState, useEffect, useCallback } from 'react';
import { api, type ChatMessage } from '../api/client';
import { wsClient } from '../api/websocket';

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);

  useEffect(() => {
    // Load history
    api.chat.history().then((history) => {
      setMessages(history);
      setLoading(false);
    });

    // Subscribe to chat updates
    const handleMessage = (data: unknown) => {
      setMessages((prev) => [...prev, data as ChatMessage]);
    };

    wsClient.on('chat', handleMessage);

    return () => {
      wsClient.off('chat', handleMessage);
    };
  }, []);

  const sendMessage = useCallback(async (content: string) => {
    setSending(true);
    try {
      // Add user message optimistically
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, userMessage]);

      await api.chat.send(content);
    } finally {
      setSending(false);
    }
  }, []);

  const clearHistory = useCallback(async () => {
    await api.chat.clear();
    setMessages([]);
  }, []);

  return { messages, loading, sending, sendMessage, clearHistory };
}
