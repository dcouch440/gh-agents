import { useEffect, useRef, useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { ChevronDown, FileText } from 'lucide-react';
import { useChat } from '../../hooks/useChat';
import { Message } from '../../components/Message';
import { ChatInput } from '../../components/ChatInput';
import { ChatSkeleton } from '../../components/Skeleton';
import { TypingIndicator } from '../../components/TypingIndicator';
import { DocPanel } from '../../components/DocPanel';
import styles from './ChatPage.module.css';

const MOCK_DOCUMENTS = [
  { id: '1', title: 'Architecture Overview' },
  { id: '2', title: 'API Reference' },
  { id: '3', title: 'Deployment Guide' },
];

const MOCK_DOC_CONTENT: Record<string, { id: string; title: string; content: string }> = {
  '1': {
    id: '1',
    title: 'Architecture Overview',
    content: '# Architecture Overview\n\nThis document describes the system architecture.\n\n## Components\n\n- **Backend**: Rust (Axum) REST API\n- **Frontend**: React + Vite\n- **Database**: PostgreSQL\n\n## Data Flow\n\nRequests flow through the API layer to the orchestration engine.',
  },
  '2': {
    id: '2',
    title: 'API Reference',
    content: '# API Reference\n\n## Endpoints\n\n### `POST /api/chat`\n\nSend a message to the chat.\n\n### `GET /api/tasks`\n\nList all tasks.\n\n### `GET /api/agents`\n\nList all agents.',
  },
  '3': {
    id: '3',
    title: 'Deployment Guide',
    content: '# Deployment Guide\n\n## Prerequisites\n\n- Docker\n- PostgreSQL 15+\n\n## Steps\n\n1. Build the container\n2. Run migrations\n3. Start the server',
  },
};

export function ChatPage() {
  const { sessionId } = useParams<{ sessionId?: string }>();
  const { messages, loading, sending, waitingForResponse, sendMessage, retryLastMessage, toolExecutions } = useChat(
    sessionId ? { sessionId } : undefined
  );
  const bottomRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [showScrollBtn, setShowScrollBtn] = useState(false);
  const [docPanelOpen, setDocPanelOpen] = useState(false);
  const [selectedDocId, setSelectedDocId] = useState<string | null>(null);

  const selectedDoc = selectedDocId ? MOCK_DOC_CONTENT[selectedDocId] ?? null : null;

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
      <div className={styles.workspace}>
        <div className={styles.chatPane}>
          <div className={styles.messageList}>
            <ChatSkeleton />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.workspace}>
      <div className={styles.chatPane}>
        <div className={styles.chatHeader}>
          <button
            className={`${styles.docToggle} ${docPanelOpen ? styles.docToggleActive : ''}`}
            onClick={() => setDocPanelOpen(!docPanelOpen)}
            title="Toggle document panel"
          >
            <FileText size={16} />
          </button>
        </div>
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
                toolExecutions={toolExecutions[message.id]}
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

        <div className={styles.inputArea}>
          <ChatInput onSend={sendMessage} disabled={sending} />
        </div>
      </div>

      <DocPanel
        isOpen={docPanelOpen}
        onClose={() => setDocPanelOpen(false)}
        document={selectedDoc}
        documents={MOCK_DOCUMENTS}
        onSelectDocument={(id) => setSelectedDocId(id)}
      />
    </div>
  );
}
