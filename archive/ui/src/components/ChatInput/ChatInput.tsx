import { useState, useRef, useCallback, forwardRef } from 'react';
import type { KeyboardEvent, ChangeEvent } from 'react';
import styles from './ChatInput.module.css';

interface ChatInputProps {
  onSend: (message: string) => void;
  onCancel?: () => void;
  disabled?: boolean;
}

const MAX_ROWS = 6;

export const ChatInput = forwardRef<HTMLDivElement, ChatInputProps>(
  ({ onSend, onCancel, disabled }, ref) => {
    const [value, setValue] = useState('');
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const adjustHeight = useCallback(() => {
      const textarea = textareaRef.current;
      if (!textarea) return;

      textarea.style.height = 'auto';
      const lineHeight = parseInt(getComputedStyle(textarea).lineHeight, 10) || 20;
      const maxHeight = lineHeight * MAX_ROWS;
      textarea.style.height = `${Math.min(textarea.scrollHeight, maxHeight)}px`;
    }, []);

    const handleSubmit = () => {
      if (value.trim() && !disabled) {
        onSend(value.trim());
        setValue('');
        if (textareaRef.current) {
          textareaRef.current.style.height = 'auto';
        }
      }
    };

    const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
      setValue(e.target.value);
      adjustHeight();
    };

    const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
      if (e.key === 'Escape' && disabled && onCancel) {
        e.preventDefault();
        onCancel();
      }
    };

    return (
      <div ref={ref} className={styles.container}>
        <div className={`${styles.inputWrapper} ${disabled ? styles.inputWrapperDisabled : ''}`}>
          <textarea
            ref={textareaRef}
            value={value}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder=">"
            disabled={disabled}
            rows={2}
            className={styles.textarea}
          />
          {disabled && (
            <span className={styles.escBadge}>Esc</span>
          )}
        </div>
      </div>
    );
  }
);

ChatInput.displayName = 'ChatInput';
