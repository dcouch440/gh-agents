import { useState, forwardRef } from 'react';
import type { KeyboardEvent } from 'react';
import { Send } from 'lucide-react';
import { Button } from '../Button';
import styles from './ChatInput.module.css';

interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
}

export const ChatInput = forwardRef<HTMLDivElement, ChatInputProps>(
  ({ onSend, disabled }, ref) => {
    const [value, setValue] = useState('');

    const handleSubmit = () => {
      if (value.trim() && !disabled) {
        onSend(value.trim());
        setValue('');
      }
    };

    const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    };

    return (
      <div ref={ref} className={styles.container}>
        <div className={styles.inputWrapper}>
          <textarea
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message..."
            disabled={disabled}
            rows={1}
            className={styles.textarea}
          />
          <Button
            onClick={handleSubmit}
            disabled={disabled || !value.trim()}
            variant="primary"
            size="md"
          >
            <Send size={20} />
          </Button>
        </div>
        <p className={styles.hint}>
          Press Enter to send, Shift+Enter for new line
        </p>
      </div>
    );
  }
);

ChatInput.displayName = 'ChatInput';
