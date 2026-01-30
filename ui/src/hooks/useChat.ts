import { useState, useEffect, useCallback } from 'react';
import { api, type ChatMessage } from '../api/client';

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

      const { message_id } = await api.chat.send(content);

      // Create a placeholder assistant message for streaming
      const assistantId = crypto.randomUUID();
      const assistantMessage: ChatMessage = {
        id: assistantId,
        role: 'assistant',
        content: '',
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, assistantMessage]);

      // Subscribe to the SSE stream for this response
      api.chat.stream(
        message_id,
        (token) => {
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === assistantId
                ? { ...msg, content: msg.content + token }
                : msg
            )
          );
        },
        () => {
          setSending(false);
        },
        () => {
          setSending(false);
        }
      );
    } catch {
      setSending(false);
    }
  }, []);

  const clearHistory = useCallback(async () => {
    await api.chat.clear();
    setMessages([]);
  }, []);

  return { messages, loading, sending, sendMessage, clearHistory };
}
