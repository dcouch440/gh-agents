import { useEffect } from 'react'
import Snackbar from '@mui/material/Snackbar'
import Alert from '@mui/material/Alert'
import { useStore } from '@/stores/lib'
import { uiStore } from '@/stores/uiStore'
import type { Toast } from '@/stores/uiStore'

function ToastItem({ toast }: { readonly toast: Toast }) {
  useEffect(() => {
    if (toast.duration === null) return
    const timer = setTimeout(() => { uiStore.dismissToast(toast.id) }, toast.duration)
    return () => { clearTimeout(timer) }
  }, [toast.id, toast.duration])

  return (
    <Alert
      severity={toast.type}
      variant="filled"
      onClose={() => { uiStore.dismissToast(toast.id) }}
      sx={{ width: '100%', mb: 1 }}
    >
      {toast.message}
    </Alert>
  )
}

function ToastStack() {
  const toasts = useStore(uiStore.store, uiStore.selectToasts)

  if (toasts.length === 0) return null

  return (
    <Snackbar
      open
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
    >
      <div>
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} />
        ))}
      </div>
    </Snackbar>
  )
}

export { ToastStack }
