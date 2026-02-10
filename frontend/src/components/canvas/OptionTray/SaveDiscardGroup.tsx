import { useState, useCallback, useEffect } from 'react'
import MuiButton from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import Tooltip from '@mui/material/Tooltip'
import Fade from '@mui/material/Fade'
import SaveOutlined from '@mui/icons-material/SaveOutlined'
import UndoOutlined from '@mui/icons-material/UndoOutlined'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'

function SaveDiscardGroup() {
  const theme = useTheme()
  const isDark = theme.palette.mode === 'dark'
  const chromeBg = theme.palette.custom.chromeBg
  const dirty = useStore(workflowStore.store, workflowStore.selectDirty)
  const [saving, setSaving] = useState(false)
  const [justSaved, setJustSaved] = useState(false)

  const handleSave = useCallback(async () => {
    setSaving(true)
    try {
      await workflowStore.saveAllDirtySteps()
      setJustSaved(true)
      setTimeout(() => {
        setJustSaved(false)
      }, 2000)
    } finally {
      setSaving(false)
    }
  }, [])

  const handleDiscard = useCallback(() => {
    void workflowStore.revertSteps()
  }, [])

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        if (dirty && !saving) {
          void handleSave()
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [dirty, saving, handleSave])

  const chromeButtonSx = {
    fontSize: 13,
    fontWeight: 600,
    textTransform: 'none' as const,
    px: 2.5,
    py: 0.75,
    minWidth: 100,
    color: '#fff',
    background: isDark
      ? `linear-gradient(135deg, ${chromeBg} 0%, ${chromeBg} 100%)`
      : `linear-gradient(135deg, ${chromeBg}dd 0%, ${chromeBg} 100%)`,
    boxShadow: `0 2px 8px ${chromeBg}33`,
    transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    '&:hover': {
      background: chromeBg,
      boxShadow: `0 4px 14px ${chromeBg}4d`,
      transform: 'translateY(-1px)',
    },
    '&:active': {
      transform: 'translateY(0) scale(0.98)',
      boxShadow: `0 2px 8px ${chromeBg}33`,
    },
    '&.Mui-disabled': {
      background: isDark
        ? 'linear-gradient(135deg, #3a3a3a 0%, #2a2a2a 100%)'
        : 'linear-gradient(135deg, #c0c0c0 0%, #a0a0a0 100%)',
      color: isDark ? 'rgba(255, 255, 255, 0.35)' : 'rgba(255, 255, 255, 0.7)',
      boxShadow: 'none',
    },
  }

  return (
    <>
      <Tooltip title="Discard changes (Esc)" TransitionComponent={Fade} enterDelay={500} placement="top">
        <span data-testid="toolbar-discard-button">
          <MuiButton
            size="small"
            variant="outlined"
            startIcon={
              <UndoOutlined
                sx={{
                  fontSize: 16,
                  transition: 'transform 0.2s ease',
                }}
              />
            }
            onClick={handleDiscard}
            disabled={!dirty || saving}
            sx={{
              fontSize: 13,
              fontWeight: 500,
              textTransform: 'none',
              borderColor: theme.palette.custom.floatingPanelBorder,
              color: theme.palette.text.primary,
              px: 2,
              py: 0.75,
              minWidth: 100,
              transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
              '&:hover': {
                borderColor: theme.palette.custom.borderHover,
                backgroundColor: theme.palette.custom.activeTintStrong,
                '& .MuiSvgIcon-root': {
                  transform: 'rotate(-30deg)',
                },
              },
              '&:active': {
                transform: 'scale(0.98)',
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
        title={!dirty && !saving ? 'All changes saved' : justSaved ? 'Saved!' : 'Save changes (\u2318S)'}
        TransitionComponent={Fade}
        enterDelay={500}
        placement="top"
      >
        <span data-testid="toolbar-save-button">
          <MuiButton
            size="small"
            variant="contained"
            startIcon={saving ? <CircularProgress size={14} thickness={5} color="inherit" /> : <SaveOutlined sx={{ fontSize: 16 }} />}
            onClick={() => {
              void handleSave()
            }}
            disabled={!dirty || saving || justSaved}
            sx={chromeButtonSx}
          >
            {saving ? 'Saving...' : !dirty ? 'Saved' : justSaved ? 'Saved' : 'Save'}
          </MuiButton>
        </span>
      </Tooltip>
    </>
  )
}

export { SaveDiscardGroup }
