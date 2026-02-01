type ErrorMessageProps = {
  message: string
  onRetry?: (() => void) | null
}

function ErrorMessage({ message, onRetry }: ErrorMessageProps) {
  return (
    <div className="error-message">
      <span>{message}</span>
      {onRetry ? (
        <button className="error-message__retry" onClick={onRetry}>
          Retry
        </button>
      ) : null}
    </div>
  )
}

export { ErrorMessage }
export type { ErrorMessageProps }
