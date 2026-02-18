import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, shallow, stepStreamStore, workflowExecutionStore } from '@/stores'
import type { SourceStreamState } from '@/stores'
import { StreamView, ToolActivityFeed, ExecutionStatusBadge, ExecutionProgress, toExecutionStatus } from '../../execution'

type LiveStreamTabProps = {
  stepId: string
}

function LiveStreamTab({ stepId }: LiveStreamTabProps) {
  const sources = useStore(stepStreamStore.store, stepStreamStore.selectSourcesForStep(stepId), shallow)
  const designerStatus = useStore(stepStreamStore.store, stepStreamStore.selectDesignerStatus)
  const stepExec = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepState(stepId))
  const execStatus = toExecutionStatus(stepExec?.status)

  const forEachProgress = stepExec?.forEachProgress ?? null
  const hasSources = sources.length > 0
  const isDesignerActive = designerStatus === 'running' || designerStatus === 'completed' || designerStatus === 'failed'

  if (!hasSources && !isDesignerActive && execStatus === 'idle') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          No live data yet. Run the workflow to see streaming output.
        </Typography>
      </Box>
    )
  }

  return (
    <Box className="nowheel nodrag nopan" sx={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Overall progress bar */}
      {forEachProgress !== null && (
        <Box sx={{ px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', flexShrink: 0 }}>
          <ExecutionProgress
            completed={forEachProgress.completed}
            total={forEachProgress.total}
            label="Items"
          />
        </Box>
      )}

      {/* Designer status row */}
      {isDesignerActive && (
        <Box sx={{ px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', flexShrink: 0, display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
            Designer
          </Typography>
          <ExecutionStatusBadge status={toExecutionStatus(designerStatus)} />
        </Box>
      )}

      {/* Source cards */}
      <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', p: 1 }}>
        {sources.map((source) => (
          <SourceCard key={source.sourceId} source={source} />
        ))}
      </Box>
    </Box>
  )
}

function SourceCard({ source }: { source: SourceStreamState }) {
  const streamStatus = source.status === 'completed' || source.status === 'failed' ? source.status : 'running'

  return (
    <Box
      sx={{
        mb: 1,
        border: 1,
        borderColor: 'divider',
        borderRadius: 1,
        overflow: 'hidden',
      }}
    >
      {/* Source header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 0.5,
          backgroundColor: 'action.hover',
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography variant="caption" sx={{ fontWeight: 600, flex: 1 }}>
          {source.sourceName}
        </Typography>
        <ExecutionStatusBadge status={toExecutionStatus(source.status)} />
      </Box>

      {/* Stream content */}
      <Box sx={{ px: 1, py: 0.5, maxHeight: 200 }}>
        <StreamView
          content={source.streamBuffer}
          status={streamStatus}
          error={source.error}
        />
      </Box>

      {/* Tool activity */}
      {source.toolUses.length > 0 && (
        <Box sx={{ px: 1.5, py: 0.5, borderTop: 1, borderColor: 'divider' }}>
          <ToolActivityFeed tools={source.toolUses} compact />
        </Box>
      )}
    </Box>
  )
}

export { LiveStreamTab }
export type { LiveStreamTabProps }
