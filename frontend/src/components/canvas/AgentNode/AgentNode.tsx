import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { DOCUMENT_NODE } from '../DocumentNode'
import { DetailLevel } from '../constants'
import { AGENT_NODE } from './constants'
import { AgentNodeHeader } from './AgentNodeHeader'
import type { AgentNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'
import { useCanvasLOD } from '../useCanvasLOD'
import { MinimalNodeShell } from '../MinimalNodeShell'
import { ResizableNodeShell, toConstraints } from '../ResizableNodeShell'

function AgentNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const detailLevel = useCanvasLOD()
  const nodeData = data as AgentNodeData
  const highlightMode = useProtocolHighlight(CanvasNodeKind.AGENT, id, nodeData.protocolStepId)
  const accentColor = AGENT_NODE.ACCENT_COLOR
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  if (detailLevel === DetailLevel.MINIMAL) {
    return (
      <Box sx={{ width: '100%', height: '100%' }}>
        <MinimalNodeShell
          label={nodeData.label}
          accentColor={accentColor}
          borderColor={highlight.borderColor}
          boxShadow={highlight.boxShadow}
        />
        <CanvasHandle type="target" position={Position.Bottom} id="agent-input" color={accentColor} variant="passive" />
        <CanvasHandle type="source" position={Position.Top} id="agent-output" color={accentColor} variant="passive" />
        <CanvasHandle type="source" position={Position.Right} id="agent-documents" color={DOCUMENT_NODE.ACCENT_COLOR} variant="passive" />
      </Box>
    )
  }

  return (
    <ResizableNodeShell
      nodeId={id}
      selected={selected === true}
      accentColor={accentColor}
      highlight={highlight}
      constraints={toConstraints(AGENT_NODE)}
      handles={
        <>
          <CanvasHandle type="target" position={Position.Bottom} id="agent-input" color={accentColor} variant="passive" />
          <CanvasHandle type="source" position={Position.Top} id="agent-output" color={accentColor} variant="passive" />
          <CanvasHandle type="source" position={Position.Right} id="agent-documents" color={DOCUMENT_NODE.ACCENT_COLOR} variant="passive" />
        </>
      }
    >
      <Box
        sx={{
          height: AGENT_NODE.HEADER_HEIGHT,
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
          backgroundColor: theme.palette.custom.bgHeader,
          flexShrink: 0,
          cursor: 'grab',
          '&:active': { cursor: 'grabbing' },
        }}
      >
        <AgentNodeHeader
          name={nodeData.label}
          roleDescription={nodeData.roleDescription}
          parentStepName={nodeData.parentStepName}
          accentColor={accentColor}
        />
      </Box>
    </ResizableNodeShell>
  )
}

const agentNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const AgentNode = memo(AgentNodeComponent, agentNodeEqual)

export { AgentNode }
