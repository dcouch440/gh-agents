import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { InputNodeIcon } from '../Icons/InputNodeIcon'
import { ProtocolBadge } from '../ProtocolBadge'
import { INPUT_NODE } from './constants'
import { InputNodeRunButton } from './InputNodeRunButton'

type InputNodeHeaderProps = {
  stepId: string
  name: string
  accentColor?: string
}

function InputNodeHeader({ stepId, name, accentColor = INPUT_NODE.ACCENT_COLOR }: InputNodeHeaderProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, width: '100%' }}>
      <Box
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <InputNodeIcon color={accentColor} size={18} />
      </Box>

      <Box sx={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
        <Typography
          sx={{
            fontSize: 13,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 10,
            fontWeight: 500,
            color: 'text.secondary',
            lineHeight: 1.2,
          }}
        >
          Editable input for each run
        </Typography>
      </Box>

      <Box className="nodrag" sx={{ flexShrink: 0 }}>
        <InputNodeRunButton stepId={stepId} />
      </Box>

      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Input" animated />
      </Box>
    </Box>
  )
}

export { InputNodeHeader }
export type { InputNodeHeaderProps }
