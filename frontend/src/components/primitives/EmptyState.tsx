type EmptyStateProps = {
  icon?: string
  message: string
}

function EmptyState({ icon, message }: EmptyStateProps) {
  return (
    <div className="empty-state">
      {icon ? <div className="empty-state__icon">{icon}</div> : null}
      <div className="empty-state__message">{message}</div>
    </div>
  )
}

export { EmptyState }
export type { EmptyStateProps }
