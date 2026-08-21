import { useMemo, useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Divider from '@mui/material/Divider'
import IconButton from '@mui/material/IconButton'
import ContentCopyRounded from '@mui/icons-material/ContentCopyRounded'
import CheckRounded from '@mui/icons-material/CheckRounded'
import { useStore } from '@/stores/lib'
import { workflowStore } from '@/stores/workflowStore'
import { workflowLiveStore } from '@/stores/workflowLiveStore'
import { dispatchStore } from '@/stores/dispatchStore'
import { Collections } from '@/utils/collections'
import { PhaseZeroSummary } from './PhaseZeroSummary'
import { DispatchAccordionRow } from './DispatchAccordionRow'
import { buildDispatchExport } from './exportDispatch'

/**
 * Dispatch tab content — PhaseZeroSummary + one accordion row per dispatch.
 *
 * Rows come from the live-state endpoint rather than the last board-submit
 * response, so they survive a refresh and are correct after a Generate (which
 * persists no submit response of its own). The server already returns them
 * newest-first, one per step.
 */
function DispatchTab() {
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const dispatches = useStore(workflowLiveStore.store, workflowLiveStore.selectDispatches)
  const [copied, setCopied] = useState(false)

  const stepNameMap = useMemo(
    () => Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? s.id.slice(0, 8)),
    [steps],
  )

  return (
    <>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', px: 1, py: 0.5 }}>
        <IconButton
          size="small"
          aria-label="Copy dispatch JSON"
          onClick={() => {
            const data = buildDispatchExport()
            void navigator.clipboard.writeText(JSON.stringify(data, null, 2)).then(() => {
              setCopied(true)
              setTimeout(() => { setCopied(false) }, 1500)
            })
          }}
        >
          {copied
            ? <CheckRounded sx={{ fontSize: 14, color: 'success.main' }} />
            : <ContentCopyRounded sx={{ fontSize: 14, color: 'text.disabled' }} />}
        </IconButton>
      </Box>
      <PhaseZeroSummary />
      <Divider />
      {dispatches.length === 0 ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', py: 4 }}>
          <Typography variant="body2" sx={{ color: 'text.disabled', fontStyle: 'italic' }}>
            No dispatches yet. Submit the board to trigger dispatch agents.
          </Typography>
        </Box>
      ) : (
        dispatches.map((d) => (
          <ConnectedRow
            key={d.executionId}
            stepId={d.stepId}
            stepName={stepNameMap.get(d.stepId) ?? d.stepId.slice(0, 8)}
            instruction={d.instruction}
          />
        ))
      )}
    </>
  )
}

// ── Internal components (isolated store subscriptions per row) ───────────────

type ConnectedRowProps = {
  readonly stepId: string
  readonly stepName: string
  readonly instruction: string
}

function ConnectedRow({ stepId, stepName, instruction }: ConnectedRowProps) {
  const entry = useStore(dispatchStore.store, dispatchStore.selectByStepId(stepId))

  return (
    <DispatchAccordionRow
      stepName={stepName}
      instruction={instruction}
      entry={entry}
    />
  )
}

export { DispatchTab }
