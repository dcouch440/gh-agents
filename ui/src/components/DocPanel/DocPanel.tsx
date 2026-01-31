import { X, FileText } from 'lucide-react';
import { MarkdownContent } from '../MarkdownContent';
import styles from './DocPanel.module.css';

interface DocPanelProps {
  isOpen: boolean;
  onClose: () => void;
  document?: { id: string; title: string; content: string } | null;
  documents?: { id: string; title: string }[];
  onSelectDocument?: (id: string) => void;
}

export function DocPanel({ isOpen, onClose, document, documents, onSelectDocument }: DocPanelProps) {
  return (
    <div className={`${styles.panel} ${!isOpen ? styles.panelHidden : ''}`}>
      <div className={styles.header}>
        <span className={styles.title}>{document?.title ?? 'Documents'}</span>
        <button className={styles.closeBtn} onClick={onClose} title="Close panel">
          <X size={14} />
        </button>
      </div>

      {documents && documents.length > 0 && (
        <div className={styles.cardList}>
          {documents.map((doc) => (
            <button
              key={doc.id}
              className={`${styles.card} ${document?.id === doc.id ? styles.cardActive : ''}`}
              onClick={() => onSelectDocument?.(doc.id)}
            >
              <FileText size={14} className={styles.cardIcon} />
              <span className={styles.cardTitle}>{doc.title}</span>
            </button>
          ))}
        </div>
      )}

      <div className={styles.body}>
        {document?.content ? (
          <MarkdownContent content={document.content} />
        ) : (
          <div className={styles.emptyState}>
            <FileText size={32} className={styles.emptyIcon} />
            <span>No document selected</span>
          </div>
        )}
      </div>
    </div>
  );
}
