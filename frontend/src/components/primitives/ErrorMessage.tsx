import { Alert, Button } from '@mui/material'

type ErrorMessageProps = {
  message: string
  onRetry?: (() => void) | null
}

function ErrorMessage({ message, onRetry }: ErrorMessageProps) {
  return (
    <Alert
      severity="error"
      sx={{ mb: 2 }}
      action={
        onRetry ? (
          <Button color="inherit" size="small" onClick={onRetry}>
            Retry
          </Button>
        ) : undefined
      }
    >
      {message}
    </Alert>
  )
}

export { ErrorMessage }
export type { ErrorMessageProps }
