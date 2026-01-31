import { useEffect, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { useChat } from '../../hooks/useChat';
import { Message } from '../../components/Message';
import { ChatInput } from '../../components/ChatInput';
import styles from './ChatPage.module.css';

export function ChatPage() {
  const { sessionId } = useParams<{ sessionId?: string }>();
  const { messages, loading, sending, sendMessage } = useChat(
    sessionId ? { sessionId } : undefined
  );
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (messages.length > 0) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  if (loading) {
    return (
      <div className={styles.loading}>
        <span>Loading...</span>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <div className={styles.messageList}>
        {messages.length === 0 ? (
          <div className={styles.emptyState}>
            <p className={styles.emptyBrand}>nexor</p>
            <p className={styles.emptyTagline}>AI agent orchestration for GitHub workflows</p>
          </div>
        ) : (
          messages.map((message) => (
            <Message key={message.id} message={message} />
          ))
        )}
        <div ref={bottomRef} />
      </div>

      <ChatInput onSend={sendMessage} disabled={sending} />
    </div>
  );
}
