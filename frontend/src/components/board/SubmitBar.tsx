import Paper from '@mui/material/Paper'
import Typography from '@mui/material/Typography'
import PublishIcon from '@mui/icons-material/Publish'
import { GradientButton } from '@/components/primitives'
import type { SubmitStatus } from '@/stores/boardStore'

type SubmitBarProps = {
  readonly onSubmit: () => void
  readonly isSubmitting: boolean
  readonly status: SubmitStatus
  readonly error: string | null
}

/**
 * Floating toolbar anchored to the bottom-center of the board.
 *
 * Renders a submit button with loading/error feedback. Entirely stateless —
 * all state flows in via props from `useBoardSubmit`.
 */
function SubmitBar({ onSubmit, isSubmitting, status, error }: SubmitBarProps) {
  return (
    <Paper
      elevation={4}
      sx={{
        position: 'absolute',
        bottom: 24,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        py: 1,
        borderRadius: 2,
        zIndex: 10,
      }}
    >
      <GradientButton
        onClick={onSubmit}
        loading={isSubmitting}
        icon={<PublishIcon sx={{ fontSize: 18 }} />}
      >
        Submit
      </GradientButton>

      {status === 'error' && error !== null && (
        <Typography variant="caption" color="error" sx={{ maxWidth: 240 }}>
          {error}
        </Typography>
      )}
    </Paper>
  )
}

export { SubmitBar }
export type { SubmitBarProps }
