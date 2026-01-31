import { useState, useEffect, useCallback } from 'react';
import { api, type ChatMessage } from '../api/client';

interface UseChatOptions {
  sessionId?: string;
}

export function useChat(options?: UseChatOptions) {
  const sessionId = options?.sessionId;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [waitingForResponse, setWaitingForResponse] = useState(false);

  useEffect(() => {
    const loadHistory = sessionId
      ? api.sessions.history(sessionId)
      : api.chat.history();
    loadHistory.then((history) => {
      setMessages(history);
      setLoading(false);
    });
  }, [sessionId]);

  const sendMessage = useCallback(async (content: string) => {
    setSending(true);
    setWaitingForResponse(true);
    try {
      // Add user message optimistically
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, userMessage]);

      const { message_id } = sessionId
        ? await api.sessions.send(sessionId, content)
        : await api.chat.send(content);

      // Create a placeholder assistant message for streaming
      const assistantId = crypto.randomUUID();
      const assistantMessage: ChatMessage = {
        id: assistantId,
        role: 'assistant',
        content: '',
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setWaitingForResponse(false);

      // Subscribe to the SSE stream for this response
      const streamFn = sessionId
        ? (onToken: (t: string) => void, onDone: () => void, onError: (e: string) => void) =>
            api.sessions.stream(sessionId, message_id, onToken, onDone, onError)
        : (onToken: (t: string) => void, onDone: () => void, onError: (e: string) => void) =>
            api.chat.stream(message_id, onToken, onDone, onError);

      streamFn(
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
      setWaitingForResponse(false);
    }
  }, [sessionId]);

  const retryLastMessage = useCallback(async () => {
    // Find the last user message
    const lastUserIdx = messages.reduce((last, m, i) => m.role === 'user' ? i : last, -1);
    if (lastUserIdx === -1) return;
    const lastUserContent = messages[lastUserIdx].content;
    // Remove the last assistant message (and the user message, we'll re-send)
    setMessages((prev) => prev.slice(0, lastUserIdx));
    await sendMessage(lastUserContent);
  }, [messages, sendMessage]);

  const clearHistory = useCallback(async () => {
    if (sessionId) {
      await api.sessions.delete(sessionId);
    } else {
      await api.chat.clear();
    }
    setMessages([]);
  }, [sessionId]);

  return { messages, loading, sending, waitingForResponse, sendMessage, clearHistory, retryLastMessage };
}
