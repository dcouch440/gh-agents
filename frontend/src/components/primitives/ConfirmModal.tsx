import type { ReactNode } from 'react'
import { Dialog, DialogTitle, DialogContent, DialogContentText, DialogActions, Alert } from '@mui/material'
import { Button } from '@/components/primitives'

type ConfirmModalProps = {
  open: boolean
  onClose: () => void
  onConfirm: () => void | Promise<void>
  title: string
  message: string | ReactNode
  confirmText?: string
  cancelText?: string
  confirmColor?: 'primary' | 'error' | 'warning' | 'success'
  loading?: boolean
  error?: string | null
}

function ConfirmModal({
  open,
  onClose,
  onConfirm,
  title,
  message,
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  confirmColor = 'primary',
  loading = false,
  error = null,
}: ConfirmModalProps) {
  const handleConfirm = () => {
    const result = onConfirm()
    if (result instanceof Promise) {
      void result
    }
  }

  return (
    <Dialog open={open} onClose={loading ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{title}</DialogTitle>

      <DialogContent>
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {typeof message === 'string' ? <DialogContentText>{message}</DialogContentText> : message}
      </DialogContent>

      <DialogActions sx={{ px: 3, py: 2 }}>
        <Button onClick={onClose} disabled={loading} variant="secondary">
          {cancelText}
        </Button>
        <Button onClick={handleConfirm} variant={confirmColor === 'error' ? 'danger' : 'primary'} disabled={loading} loading={loading}>
          {confirmText}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export { ConfirmModal }
export type { ConfirmModalProps }
