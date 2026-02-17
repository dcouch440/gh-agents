import { memo } from 'react'
import { Position } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { useStore, stepStreamStore } from '@/stores'
import { CanvasHandle } from '../CanvasHandle'
import { DOCUMENT_NODE } from '../DocumentNode'
import { DetailLevel } from '../constants'
import { ProtocolBadge } from '../ProtocolBadge'
import { NodeHeader, ExecutionStatusBadge } from '../execution'
import { AGENT_NODE } from './constants'
import { AgentNodeContent } from './AgentNodeContent'
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

  // Subscribe to status only (infrequent) — AgentNodeContent subscribes to the full stream
  const sourceStatus = useStore(
    stepStreamStore.store,
    (s) => s.sources[nodeData.rosterAgentId]?.status ?? 'idle',
  )

  const isActive = sourceStatus !== 'idle'

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
      {/* Header */}
      <Box
        sx={{
          height: AGENT_NODE.HEADER_HEIGHT,
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
          backgroundColor: theme.palette.custom.bgHeader,
          borderBottom: 1,
          borderColor: 'divider',
          flexShrink: 0,
          cursor: 'grab',
          '&:active': { cursor: 'grabbing' },
        }}
      >
        <NodeHeader
          icon={<SmartToyOutlined sx={{ fontSize: 18, color: accentColor }} />}
          title={nodeData.label}
          subtitle={nodeData.roleDescription || nodeData.parentStepName}
          accentColor={accentColor}
          badge={
            isActive
              ? <ExecutionStatusBadge status={sourceStatus === 'completed' ? 'completed' : sourceStatus === 'failed' ? 'failed' : 'running'} />
              : <ProtocolBadge color={accentColor} label="Agent" />
          }
        />
      </Box>

      {/* Content */}
      <AgentNodeContent
        rosterAgentId={nodeData.rosterAgentId}
        roleDescription={nodeData.roleDescription}
        capabilities={nodeData.capabilities}
      />
    </ResizableNodeShell>
  )
}

const agentNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected && prev.id === next.id && nodeDataEqual(prev.data, next.data)

const AgentNode = memo(AgentNodeComponent, agentNodeEqual)

export { AgentNode }
