import { Box, Typography } from '@mui/material'
import { StatusBadge, Button } from '@/components/primitives'
import type { ToolRouter } from '@/types'

type RouterInfoCardProps = {
  router: ToolRouter
  onEdit: () => void
  onDelete: () => void
  onManageTools: () => void
}

function RouterInfoCard({ router, onEdit, onDelete, onManageTools }: RouterInfoCardProps) {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mb: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
            {router.name}
          </Typography>
          <StatusBadge
            label={router.is_active ? 'Active' : 'Inactive'}
            variant={router.is_active ? 'success' : 'neutral'}
          />
        </Box>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button variant="secondary" size="small" onClick={onManageTools}>
            Tools
          </Button>
          <Button variant="secondary" size="small" onClick={onEdit}>
            Edit
          </Button>
          <Button variant="danger" size="small" onClick={onDelete}>
            Delete
          </Button>
        </Box>
      </Box>

      {router.description ? (
        <Typography variant="body2" color="text.secondary">
          {router.description}
        </Typography>
      ) : null}

      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 2 }}>
        <Box>
          <Typography variant="caption" color="text.secondary" component="div">
            Model
          </Typography>
          <Typography variant="body2">{router.model_id}</Typography>
        </Box>
        <Box>
          <Typography variant="caption" color="text.secondary" component="div">
            Level
          </Typography>
          <Typography variant="body2">{router.level}</Typography>
        </Box>
      </Box>

      <Box>
        <Typography variant="caption" color="text.secondary" component="div" sx={{ mb: 0.5 }}>
          System Prompt
        </Typography>
        <Box sx={{ p: 1.5, bgcolor: 'background.default', borderRadius: 1, border: 1, borderColor: 'divider' }}>
          <Typography
            variant="body2"
            component="pre"
            sx={{ whiteSpace: 'pre-wrap', fontFamily: 'monospace', m: 0, maxHeight: 120, overflow: 'auto' }}
          >
            {router.system_prompt}
          </Typography>
        </Box>
      </Box>
    </Box>
  )
}

export { RouterInfoCard }
export type { RouterInfoCardProps }
