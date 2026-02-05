import { Button, CircularProgress } from '@mui/material'

type ApproveButtonProps = {
  onApprove: () => void
  loading: boolean
  disabled: boolean
}

function ApproveButton({ onApprove, loading, disabled }: ApproveButtonProps) {
  return (
    <Button
      variant="contained"
      color="success"
      onClick={onApprove}
      disabled={loading || disabled}
      startIcon={loading ? <CircularProgress size={16} color="inherit" /> : null}
      sx={{ fontWeight: 600 }}
    >
      {loading ? 'Approving...' : 'Approve'}
    </Button>
  )
}

export { ApproveButton }
export type { ApproveButtonProps }
