type SpinnerSize = 'sm' | 'md' | 'lg'

type LoadingSpinnerProps = {
  size?: SpinnerSize
  centered?: boolean
}

function LoadingSpinner({ size = 'md', centered = false }: LoadingSpinnerProps) {
  const spinner = <div className={`spinner spinner--${size}`} />

  if (centered) {
    return <div className="spinner-container">{spinner}</div>
  }

  return spinner
}

export { LoadingSpinner }
export type { SpinnerSize, LoadingSpinnerProps }
