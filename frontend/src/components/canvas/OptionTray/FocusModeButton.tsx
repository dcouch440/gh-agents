import IconButton from '@mui/material/IconButton'
import { Tooltip } from '@/components/primitives/Tooltip'
import FullscreenOutlined from '@mui/icons-material/FullscreenOutlined'

type FocusModeButtonProps = {
  onClick: () => void
}

function FocusModeButton({ onClick }: FocusModeButtonProps) {
  return (
    <Tooltip title="Focus Mode (Alt+F)" placement="top">
      <IconButton
        onClick={onClick}
        size="small"
        sx={{
          width: 32,
          height: 32,
          color: 'text.secondary',
          '&:hover': { color: 'text.primary' },
        }}
      >
        <FullscreenOutlined sx={{ fontSize: 18 }} />
      </IconButton>
    </Tooltip>
  )
}

export { FocusModeButton }
