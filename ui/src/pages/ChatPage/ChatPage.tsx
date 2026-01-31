import { useEffect, useRef, useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { ChevronDown } from 'lucide-react';
import { useChat } from '../../hooks/useChat';
import { Message } from '../../components/Message';
import { ChatInput } from '../../components/ChatInput';
import { ChatSkeleton } from '../../components/Skeleton';
import { TypingIndicator } from '../../components/TypingIndicator';
import styles from './ChatPage.module.css';

export function ChatPage() {
  const { sessionId } = useParams<{ sessionId?: string }>();
  const { messages, loading, sending, waitingForResponse, sendMessage, retryLastMessage } = useChat(
    sessionId ? { sessionId } : undefined
  );
  const bottomRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [showScrollBtn, setShowScrollBtn] = useState(false);

  useEffect(() => {
    if (messages.length > 0 && !showScrollBtn) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, showScrollBtn]);

  const handleScroll = useCallback(() => {
    const el = listRef.current;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    setShowScrollBtn(distanceFromBottom > 200);
  }, []);

  const scrollToBottom = () => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  const lastAssistantIdx = messages.reduce((last: number, m, i) => m.role === 'assistant' ? i : last, -1);

  if (loading) {
    return (
      <div className={styles.container}>
        <div className={styles.messageList}>
          <ChatSkeleton />
        </div>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <div className={styles.messageList} ref={listRef} onScroll={handleScroll}>
        {messages.length === 0 ? (
          <div className={styles.emptyState}>
            <p className={styles.emptyBrand}>nexor</p>
            <p className={styles.emptyTagline}>AI agent orchestration for GitHub workflows</p>
          </div>
        ) : (
          messages.map((message, idx) => (
            <Message
              key={message.id}
              message={message}
              streaming={sending && idx === messages.length - 1 && message.role === 'assistant'}
              isLast={idx === lastAssistantIdx}
              onRetry={idx === lastAssistantIdx ? retryLastMessage : undefined}
            />
          ))
        )}
        {waitingForResponse && <TypingIndicator />}
        <div ref={bottomRef} />

        {showScrollBtn && (
          <button className={styles.scrollBtn} onClick={scrollToBottom}>
            <ChevronDown size={18} />
          </button>
        )}
      </div>

      <ChatInput onSend={sendMessage} disabled={sending} />
    </div>
  );
}
