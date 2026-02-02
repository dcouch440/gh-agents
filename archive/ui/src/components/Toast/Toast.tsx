import { useEffect, useState } from 'react';
import { CheckCircle, XCircle, Info, X } from 'lucide-react';
import { useToastStore, type Toast as ToastType } from '../../store';
import styles from './Toast.module.css';

function ToastItem({ toast, onDismiss }: { toast: ToastType; onDismiss: (id: string) => void }) {
  const [exiting, setExiting] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setExiting(true);
      setTimeout(() => onDismiss(toast.id), 200);
    }, 4000);
    return () => clearTimeout(timer);
  }, [toast.id, onDismiss]);

  const handleDismiss = () => {
    setExiting(true);
    setTimeout(() => onDismiss(toast.id), 200);
  };

  const Icon = toast.type === 'success' ? CheckCircle : toast.type === 'error' ? XCircle : Info;
  const iconClass = toast.type === 'success' ? styles.iconSuccess : toast.type === 'error' ? styles.iconError : styles.iconInfo;

  return (
    <div className={`${styles.toast} ${exiting ? styles.exiting : ''}`}>
      <Icon size={16} className={`${styles.icon} ${iconClass}`} />
      <span className={styles.message}>{toast.message}</span>
      <button className={styles.close} onClick={handleDismiss}>
        <X size={14} />
      </button>
    </div>
  );
}

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className={styles.container}>
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={removeToast} />
      ))}
    </div>
  );
}
