import IconButton from '@mui/material/IconButton'
import { Tooltip } from '@/components/primitives/Tooltip'
import AutoFixHighOutlined from '@mui/icons-material/AutoFixHighOutlined'

type AutoLayoutButtonProps = {
  onClick: () => void
}

function AutoLayoutButton({ onClick }: AutoLayoutButtonProps) {
  return (
    <Tooltip title="Auto Layout" placement="top">
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
        <AutoFixHighOutlined sx={{ fontSize: 18 }} />
      </IconButton>
    </Tooltip>
  )
}

export { AutoLayoutButton }
