import type { NodeProps } from '@xyflow/react'
import { Handle, Position } from '@xyflow/react'
import { Box, Typography } from '@mui/material'
import type { StepNode } from './types'
import { getStatusColor, nodeBoxSx } from './nodeHelpers'

function ForEachStepNode({ data, selected }: NodeProps<StepNode>) {
  const statusColor = getStatusColor(data.executionState?.status)
  const progress = data.executionState?.forEachProgress

  return (
    <>
      <Handle type="target" position={Position.Top} />
      <Box sx={nodeBoxSx(data, !!selected, 'secondary.main')}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
          <Box sx={{ width: 8, height: 8, borderRadius: '50%', bgcolor: statusColor, flexShrink: 0 }} />
          <Typography variant="body2" fontWeight="bold" noWrap>
            {data.name}
          </Typography>
        </Box>
        <Typography variant="caption" color="secondary.main" fontWeight="bold">
          FOR EACH
        </Typography>
        {data.forEachLabelField && (
          <Typography variant="caption" display="block" color="text.secondary">
            Label: {data.forEachLabelField}
          </Typography>
        )}
        {progress && (
          <Typography variant="caption" display="block" color="text.secondary">
            {progress.completed} / {progress.total}
          </Typography>
        )}
      </Box>
      <Handle type="source" position={Position.Bottom} />
    </>
  )
}

export { ForEachStepNode }
