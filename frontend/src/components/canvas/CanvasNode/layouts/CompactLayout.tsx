import { Position } from '@xyflow/react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { CanvasHandle } from '../../CanvasHandle'
import type { CompactNodeData } from '../types'
import type { NodeHighlightOutput } from '../../nodeHighlightStyles'

type CompactLayoutProps = {
  data: CompactNodeData
  accentColor: string
  highlight: NodeHighlightOutput
}

function CompactLayout({ data, accentColor, highlight }: CompactLayoutProps) {
  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '10px',
        backgroundColor: 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        px: 1.25,
        cursor: 'grab',
        '&:active': { cursor: 'grabbing' },
      }}
    >
      <Box
        sx={{
          width: 32,
          height: 32,
          borderRadius: '8px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <SmartToyOutlined sx={{ fontSize: 18, color: accentColor }} />
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: 12,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            lineHeight: 1.3,
          }}
        >
          {data.label}
        </Typography>
        {data.templateName !== null && (
          <Typography
            sx={{
              fontSize: 9,
              color: 'text.disabled',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              lineHeight: 1.2,
            }}
          >
            {data.templateName}
          </Typography>
        )}
      </Box>

      <CanvasHandle type="target" position={Position.Left} color={accentColor} variant="small" />
      <CanvasHandle type="source" position={Position.Right} color={accentColor} variant="small" />
    </Box>
  )
}

export { CompactLayout }
export type { CompactLayoutProps }
