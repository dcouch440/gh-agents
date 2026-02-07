import { useStore, reviewQueueStore } from '@/stores'
import { NotificationSnackbar } from '@/components/primitives/NotificationSnackbar'

function ReviewQueueNotification() {
  const notification = useStore(reviewQueueStore.store, reviewQueueStore.selectNotification)

  return (
    <NotificationSnackbar
      open={notification !== null}
      message={notification?.message ?? ''}
      onClose={reviewQueueStore.dismissNotification}
      severity="warning"
    />
  )
}

export { ReviewQueueNotification }
