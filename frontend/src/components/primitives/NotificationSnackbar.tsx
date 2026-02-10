import { Snackbar, Alert } from '@mui/material'

type NotificationSnackbarProps = {
  open: boolean
  message: string
  onClose: () => void
  severity?: 'info' | 'success' | 'warning' | 'error'
}

function NotificationSnackbar({ open, message, onClose, severity = 'info' }: NotificationSnackbarProps) {
  return (
    <Snackbar open={open} autoHideDuration={6000} onClose={onClose} anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}>
      <Alert onClose={onClose} severity={severity} variant="filled" sx={{ width: '100%' }}>
        {message}
      </Alert>
    </Snackbar>
  )
}

export { NotificationSnackbar }
export type { NotificationSnackbarProps }
