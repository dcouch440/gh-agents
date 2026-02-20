import { useCallback } from 'react'
import { Position } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore, shareStore } from '@/stores'
import { SharePickerPanel } from '../../SharePickerPanel'
import { CanvasHandle } from '../../CanvasHandle'
import { ProtocolBadge } from '../../ProtocolBadge'
import { NodeHeader } from '../../execution'
import { ResizableNodeShell } from '../../ResizableNodeShell'
import { ContextNodeIcon } from '../../Icons/ContextNodeIcon'
import { InputNodeIcon } from '../../Icons/InputNodeIcon'
import { ContextNodeContent } from '../content/ContextNodeContent'
import { InputNodeRunButton } from '../content/InputNodeRunButton'
import { VARIANT_CONFIGS } from '../registry'
import type { EditorNodeData } from '../types'
import type { NodeHighlightOutput } from '../../nodeHighlightStyles'

const HEADER_HEIGHT = 52

type EditorLayoutProps = {
  nodeId: string
  data: EditorNodeData
  selected: boolean
  accentColor: string
  highlight: NodeHighlightOutput
}

function EditorLayout({ nodeId, data, selected, accentColor, highlight }: EditorLayoutProps) {
  const theme = useTheme()
  const isInput = data.variant === 'input'
  const isShareSource = useStore(shareStore.store, (s) => s.active && s.sourceStepId === nodeId)

  const handleContentChange = useCallback(
    (value: string) => {
      workflowStore.patchStepLocal(nodeId, { prompt_template: value })
    },
    [nodeId],
  )

  // Editor variants always have constraints defined in the registry
  const constraints = VARIANT_CONFIGS[data.variant].constraints!

  const icon = isInput
    ? <InputNodeIcon color={accentColor} size={18} />
    : <ContextNodeIcon color={accentColor} size={18} />

  const subtitle = isInput ? 'Editable input for each run' : 'Injected directly in every agent'
  const badgeLabel = isInput ? 'Input' : 'Context'

  return (
    <ResizableNodeShell
      nodeId={nodeId}
      selected={selected}
      accentColor={accentColor}
      highlight={highlight}
      constraints={constraints}
      handles={<CanvasHandle type="source" position={Position.Bottom} color={accentColor} />}
    >
      {/* Header — draggable area */}
      <Box
        sx={{
          height: HEADER_HEIGHT,
          overflow: 'hidden',
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          backgroundColor: theme.palette.custom.screenBg,
          flexShrink: 0,
          cursor: 'grab',
          '&:active': { cursor: 'grabbing' },
        }}
      >
        <NodeHeader
          icon={icon}
          title={data.label}
          subtitle={subtitle}
          accentColor={accentColor}
          actions={isInput ? <Box className="nodrag"><InputNodeRunButton stepId={nodeId} /></Box> : undefined}
          badge={<ProtocolBadge color={accentColor} label={badgeLabel} animated />}
        />
      </Box>

      {/* Content area — interactive, no drag */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
        {isShareSource ? (
          <SharePickerPanel stepId={nodeId} />
        ) : (
          <ContextNodeContent content={data.content} accentColor={accentColor} onChange={handleContentChange} />
        )}
      </Box>
    </ResizableNodeShell>
  )
}

export { EditorLayout }
export type { EditorLayoutProps }
