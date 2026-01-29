import type { ChatMessage } from '../../api/client';
import { MarkdownContent } from '../MarkdownContent';
import styles from './Message.module.css';

interface MessageProps {
  message: ChatMessage;
  streaming?: boolean;
}

export function Message({ message, streaming }: MessageProps) {
  const isUser = message.role === 'user';

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
      <div className={`${styles.body} ${isUser ? styles.bodyUser : styles.bodyAssistant}`}>
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
  );
}
