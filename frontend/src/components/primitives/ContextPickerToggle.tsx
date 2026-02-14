import IconButton from '@mui/material/IconButton'
import AddLinkOutlined from '@mui/icons-material/AddLinkOutlined'
import { Tooltip } from './Tooltip'
import { useStore, contextPickerStore } from '@/stores'

type ContextPickerToggleProps = {
  stepId: string
}

function ContextPickerToggle({ stepId }: ContextPickerToggleProps) {
  const isActive = useStore(contextPickerStore.store, contextPickerStore.selectActive)

  const handleClick = () => {
    if (isActive) {
      contextPickerStore.deactivate()
    } else {
      contextPickerStore.activate(stepId)
    }
  }

  return (
    <Tooltip title={isActive ? 'Exit context picking' : 'Pick context to send'}>
      <IconButton
        size="small"
        onClick={handleClick}
        sx={{
          p: 0.25,
          color: isActive ? 'primary.main' : 'text.secondary',
        }}
      >
        <AddLinkOutlined sx={{ fontSize: 14 }} />
      </IconButton>
    </Tooltip>
  )
}

export { ContextPickerToggle }
export type { ContextPickerToggleProps }
