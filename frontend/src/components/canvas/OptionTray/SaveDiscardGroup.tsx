import { useCallback, useEffect } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import SaveOutlined from '@mui/icons-material/SaveOutlined'
import UndoOutlined from '@mui/icons-material/UndoOutlined'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { TRAY_BUTTON_CONTAINED_SX } from './constants'

type SaveDiscardGroupProps = {
  autoSaveFlush: () => void
  autoSaveSaving: boolean
}

function SaveDiscardGroup({ autoSaveFlush, autoSaveSaving }: SaveDiscardGroupProps) {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)

  const handleSave = useCallback(() => {
    autoSaveFlush()
  }, [autoSaveFlush])

  const handleDiscard = useCallback(() => {
    void workflowStore.revertSteps()
  }, [])

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        if (dirty && !autoSaveSaving) {
          handleSave()
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [dirty, autoSaveSaving, handleSave])

  const chromeBg = theme.palette.custom.chromeBg

  const saveLabel = autoSaveSaving ? 'Saving' : dirty ? 'Save' : 'Saved'
  const saveTooltip = autoSaveSaving
    ? 'Saving...'
    : dirty
      ? 'Save changes (\u2318S)'
      : 'All changes saved'

  return (
    <>
      <Tooltip title="Discard changes (Esc)" TransitionComponent={Fade} enterDelay={500} placement="top">
        <span data-testid="toolbar-discard-button">
          <MuiButton
            size="small"
            variant="outlined"
            startIcon={<UndoOutlined sx={{ fontSize: 16 }} />}
            onClick={handleDiscard}
            disabled={!dirty || autoSaveSaving}
            sx={{
              fontSize: 13,
              fontWeight: 500,
              textTransform: 'none',
              borderColor: theme.palette.custom.floatingPanelBorder,
              color: theme.palette.text.primary,
              px: 2,
              py: 0.75,
              minWidth: 100,
              '&:hover': {
                borderColor: theme.palette.custom.borderHover,
                backgroundColor: theme.palette.custom.activeTintStrong,
              },
              '&.Mui-disabled': {
                borderColor: theme.palette.divider,
                color: theme.palette.text.disabled,
              },
            }}
          >
            Discard
          </MuiButton>
        </span>
      </Tooltip>

      <Tooltip
        title={saveTooltip}
        TransitionComponent={Fade}
        enterDelay={500}
        placement="top"
      >
        <span data-testid="toolbar-save-button">
          <MuiButton
            size="small"
            variant="contained"
            startIcon={autoSaveSaving ? <CircularProgress size={14} thickness={5} color="inherit" /> : <SaveOutlined sx={{ fontSize: 16 }} />}
            onClick={handleSave}
            disabled={!dirty || autoSaveSaving}
            sx={{
              ...TRAY_BUTTON_CONTAINED_SX,
              minWidth: 100,
              backgroundColor: chromeBg,
              '&:hover': { backgroundColor: chromeBg, opacity: 0.9, boxShadow: 'none' },
              '&.Mui-disabled': {
                backgroundColor: isDark ? '#2a2a2a' : '#b0b0b0',
                color: isDark ? 'rgba(255, 255, 255, 0.35)' : 'rgba(255, 255, 255, 0.7)',
                boxShadow: 'none',
              },
            }}
          >
            {saveLabel}
          </MuiButton>
        </span>
      </Tooltip>
    </>
  )
}

export { SaveDiscardGroup }
