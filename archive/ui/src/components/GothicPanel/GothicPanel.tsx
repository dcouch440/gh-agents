import styles from './GothicPanel.module.css';

interface GothicPanelProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
  variant?: 'default' | 'highlight' | 'danger';
}

export function GothicPanel({ title, children, className = '', variant = 'default' }: GothicPanelProps) {
  const variantClass = variant !== 'default' ? styles[variant] : '';

  return (
    <div className={`${styles.panel} ${variantClass} ${className}`}>
      <span className={styles.cornerBL} />
      <span className={styles.cornerBR} />
      {title && <div className={styles.title}>{title}</div>}
      {children}
    </div>
  );
}
