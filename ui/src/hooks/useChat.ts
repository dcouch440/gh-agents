import { useState, useEffect, useCallback } from 'react';
import { api, type ChatMessage } from '../api/client';

export interface ToolExecution {
  name: string;
  id: string;
  status: 'running' | 'done';
}

interface UseChatOptions {
  sessionId: string;
}

export function useChat(options: UseChatOptions) {
  const { sessionId } = options;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [waitingForResponse, setWaitingForResponse] = useState(false);
  const [toolExecutions, setToolExecutions] = useState<Record<string, ToolExecution[]>>({});
  const [activeDocId, setActiveDocId] = useState<string | null>(null);
  const [docUpdated, setDocUpdated] = useState(0);

  useEffect(() => {
    api.sessions.history(sessionId).then((history) => {
      setMessages(history);
      setLoading(false);
    });
  }, [sessionId]);

  const sendMessage = useCallback(async (content: string) => {
    setSending(true);
    setWaitingForResponse(true);
    try {
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, userMessage]);

      const { message_id } = await api.sessions.send(sessionId, content);

      const assistantId = crypto.randomUUID();
      const assistantMessage: ChatMessage = {
        id: assistantId,
        role: 'assistant',
        content: '',
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setWaitingForResponse(false);

      const onToken = (token: string) => {
        setMessages((prev) =>
          prev.map((msg) =>
            msg.id === assistantId
              ? { ...msg, content: msg.content + token }
              : msg
          )
        );
      };

      const onDone = () => {
        setSending(false);
      };

      const onError = () => {
        setSending(false);
      };

      const onToolStart = (data: { name: string; id: string }) => {
        setToolExecutions((prev) => ({
          ...prev,
          [assistantId]: [
            ...(prev[assistantId] ?? []),
            { name: data.name, id: data.id, status: 'running' },
          ],
        }));
      };

      const onDocUpdate = (data: { doc_id: string; title: string }) => {
        setActiveDocId(data.doc_id);
        setDocUpdated((prev) => prev + 1);
      };

      const onToolEnd = (data: { name: string; id: string }) => {
        setToolExecutions((prev) => ({
          ...prev,
          [assistantId]: (prev[assistantId] ?? []).map((t) =>
            t.id === data.id ? { ...t, status: 'done' as const } : t
          ),
        }));
      };

      api.sessions.stream(sessionId, message_id, onToken, onDone, onError, onToolStart, onToolEnd, onDocUpdate);
    } catch {
      setSending(false);
      setWaitingForResponse(false);
    }
  }, [sessionId]);

  const retryLastMessage = useCallback(async () => {
    const lastUserIdx = messages.reduce((last, m, i) => m.role === 'user' ? i : last, -1);
    if (lastUserIdx === -1) return;
    const lastUserContent = messages[lastUserIdx].content;
    setMessages((prev) => prev.slice(0, lastUserIdx));
    await sendMessage(lastUserContent);
  }, [messages, sendMessage]);

  const clearHistory = useCallback(async () => {
    await api.sessions.delete(sessionId);
    setMessages([]);
    setToolExecutions({});
  }, [sessionId]);

  return { messages, loading, sending, waitingForResponse, sendMessage, clearHistory, retryLastMessage, toolExecutions, activeDocId, docUpdated };
}
