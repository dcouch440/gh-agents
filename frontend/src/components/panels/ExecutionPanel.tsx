import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import { EmptyState } from '@/components/primitives'

function ExecutionPanel() {
  return (
    <EmptyState
      icon={<PlayArrowOutlined fontSize="large" />}
      message="Execution view coming soon"
    />
  )
}

export { ExecutionPanel }
