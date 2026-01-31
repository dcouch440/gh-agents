import styles from './TypingIndicator.module.css';

export function TypingIndicator() {
  return (
    <div className={styles.container}>
      <div className={styles.dots}>
        <span className={styles.dot} />
        <span className={styles.dot} />
        <span className={styles.dot} />
      </div>
      <span className={styles.cancelHint}>esc to cancel</span>
    </div>
  );
}
