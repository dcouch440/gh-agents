import IconButton from '@mui/material/IconButton'
import DeleteOutlined from '@mui/icons-material/DeleteOutlined'

type ChatHeaderProps = {
  onClear: () => void
  disabled: boolean
}

function ChatHeader({ onClear, disabled }: ChatHeaderProps) {
  return (
    <IconButton
      className="nodrag"
      size="small"
      onClick={onClear}
      disabled={disabled}
      sx={{
        position: 'absolute',
        top: 4,
        right: 8,
        zIndex: 1,
        p: 0.25,
        opacity: 0.5,
        '&:hover': { opacity: 1 },
      }}
    >
      <DeleteOutlined sx={{ fontSize: 14 }} />
    </IconButton>
  )
}

export { ChatHeader }
export type { ChatHeaderProps }
