import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import CircularProgress from '@mui/material/CircularProgress'
import RefreshOutlined from '@mui/icons-material/RefreshOutlined'
import { useStepDebugLog } from '@/hooks/useStepDebugLog'
import { MessageList } from '@/components/chat'

type DebugLogTabProps = {
  stepId: string
}

function DebugLogTab({ stepId }: DebugLogTabProps) {
  const { messages, isLoading, error, refresh } = useStepDebugLog(stepId)

  if (isLoading && messages.length === 0) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 0.5,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography variant="body2" sx={{ fontWeight: 500, color: 'text.secondary' }}>
          Debug
        </Typography>
        <Tooltip title="Refresh">
          <span>
            <IconButton size="small" onClick={refresh} disabled={isLoading}>
              <RefreshOutlined fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </Box>

      {error ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
          <Typography variant="body2" color="error">
            {error}
          </Typography>
        </Box>
      ) : (
        <MessageList
          messages={messages}
          emptyMessage="No debug data available yet."
        />
      )}
    </Box>
  )
}

export { DebugLogTab }
export type { DebugLogTabProps }
