import styles from './Skeleton.module.css';

interface SkeletonProps {
  width?: string;
  height?: string;
}

export function Skeleton({ width = '100%', height = '0.75rem' }: SkeletonProps) {
  return <div className={styles.skeleton} style={{ width, height }} />;
}

export function MessageSkeleton() {
  return (
    <div className={styles.messageSkeleton}>
      <div className={styles.skeletonHeader}>
        <Skeleton width="3rem" height="0.75rem" />
        <Skeleton width="4rem" height="0.625rem" />
      </div>
      <div className={styles.skeletonLines}>
        <Skeleton width="85%" height="0.75rem" />
        <Skeleton width="70%" height="0.75rem" />
        <Skeleton width="40%" height="0.75rem" />
      </div>
    </div>
  );
}

export function ChatSkeleton() {
  return (
    <div>
      <MessageSkeleton />
      <MessageSkeleton />
      <MessageSkeleton />
    </div>
  );
}
