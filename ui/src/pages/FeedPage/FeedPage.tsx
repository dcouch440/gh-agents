import { useEffect, useRef, useState } from 'react';
import { useFeed } from '../../hooks/useFeed';
import { FeedItem } from '../../components/FeedItem';
import { ArrowDown } from 'lucide-react';
import styles from './FeedPage.module.css';

export function FeedPage() {
  const { items } = useFeed();
  const containerRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [newCount, setNewCount] = useState(0);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(isAtBottom);
    if (isAtBottom) setNewCount(0);
  };

  useEffect(() => {
    if (autoScroll) {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    } else {
      setNewCount((prev) => prev + 1);
    }
  }, [items, autoScroll]);

  const scrollToBottom = () => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    setAutoScroll(true);
    setNewCount(0);
  };

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h2 className={styles.title}>Agent Activity</h2>
        <span className={styles.count}>
          {items.length} events
        </span>
      </div>

      <div className={styles.contentWrapper}>
        {items.length === 0 ? (
          <div className={styles.emptyState}>
            <p className={styles.emptyTitle}>No agent activity yet</p>
            <p className={styles.emptySubtitle}>Start a task to see agents working</p>
          </div>
        ) : (
          <div
            ref={containerRef}
            onScroll={handleScroll}
            className={styles.feedList}
          >
            {items.map((item) => (
              <FeedItem key={item.id} item={item} />
            ))}
            <div ref={bottomRef} />
          </div>
        )}

        {!autoScroll && newCount > 0 && (
          <button
            onClick={scrollToBottom}
            className={styles.newEventsButton}
          >
            <ArrowDown size={16} />
            {newCount} new {newCount === 1 ? 'event' : 'events'}
          </button>
        )}
      </div>
    </div>
  );
}
