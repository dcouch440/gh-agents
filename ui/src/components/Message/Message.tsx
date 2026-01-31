import { useState } from 'react';
import { Copy, Check, RotateCcw } from 'lucide-react';
import type { ChatMessage } from '../../api/client';
import type { ToolExecution } from '../../hooks/useChat';
import { useToastStore } from '../../store';
import { MarkdownContent } from '../MarkdownContent';
import { ToolActivity } from '../ToolActivity';
import styles from './Message.module.css';

interface MessageProps {
  message: ChatMessage;
  streaming?: boolean;
  isLast?: boolean;
  onRetry?: () => void;
  toolExecutions?: ToolExecution[];
}

export function Message({ message, streaming, isLast, onRetry, toolExecutions }: MessageProps) {
  const isUser = message.role === 'user';
  const [copied, setCopied] = useState(false);
  const addToast = useToastStore((s) => s.addToast);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message.content);
    setCopied(true);
    addToast('Copied to clipboard', 'success');
    setTimeout(() => setCopied(false), 2000);
  };

  const bodyClasses = [
    styles.body,
    isUser ? styles.bodyUser : styles.bodyAssistant,
    !isUser && streaming ? styles.bodyStreaming : '',
    !isUser && !streaming && message.content ? styles.bodyComplete : '',
  ].filter(Boolean).join(' ');

  return (
    <div className={styles.turn}>
      <div className={styles.header}>
        <span className={`${styles.role} ${isUser ? styles.roleUser : styles.roleAssistant}`}>
          {isUser ? 'You' : 'nexor'}
        </span>
        <span className={styles.timestamp}>
          {new Date(message.timestamp).toLocaleTimeString()}
        </span>
      </div>
      <div className={bodyClasses}>
        {isUser ? (
          <p className={styles.text}>{message.content}</p>
        ) : (
          <>
            {toolExecutions && toolExecutions.length > 0 && (
              <ToolActivity executions={toolExecutions} />
            )}
            <MarkdownContent content={message.content} />
            {streaming && <span className={styles.cursor} />}
          </>
        )}
      </div>
      {message.content && !streaming && (
        <div className={styles.actions}>
          <button className={styles.actionBtn} onClick={handleCopy} title="Copy message">
            {copied ? <Check size={14} /> : <Copy size={14} />}
          </button>
          {!isUser && isLast && onRetry && (
            <button className={styles.actionBtn} onClick={onRetry} title="Retry">
              <RotateCcw size={14} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
