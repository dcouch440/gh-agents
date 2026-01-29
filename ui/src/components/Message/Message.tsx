import type { ChatMessage } from '../../api/client';
import { User, Bot } from 'lucide-react';
import { MarkdownContent } from '../MarkdownContent';
import styles from './Message.module.css';

interface MessageProps {
  message: ChatMessage;
  streaming?: boolean;
}

export function Message({ message, streaming }: MessageProps) {
  const isUser = message.role === 'user';

  return (
    <div className={`${styles.message} ${isUser ? styles.messageUser : styles.messageBot}`}>
      <div className={`${styles.avatar} ${isUser ? styles.avatarUser : styles.avatarBot}`}>
        {isUser ? <User size={16} /> : <Bot size={16} />}
      </div>

      <div className={styles.content}>
        <div className={styles.header}>
          <span className={styles.role}>{isUser ? 'You' : 'Orchestrator'}</span>
          <span className={styles.timestamp}>
            {new Date(message.timestamp).toLocaleTimeString()}
          </span>
        </div>
        <div className={`${styles.bubble} ${isUser ? styles.bubbleUser : styles.bubbleBot}`}>
          {isUser ? (
            <p className={styles.text}>{message.content}</p>
          ) : (
            <>
              <MarkdownContent content={message.content} />
              {streaming && <span className={styles.cursor} />}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
