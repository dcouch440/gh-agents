import { useMemo, useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Divider from '@mui/material/Divider'
import IconButton from '@mui/material/IconButton'
import ContentCopyRounded from '@mui/icons-material/ContentCopyRounded'
import CheckRounded from '@mui/icons-material/CheckRounded'
import { useStore } from '@/stores/lib'
import { boardStore } from '@/stores/boardStore'
import { workflowStore } from '@/stores/workflowStore'
import { dispatchStore } from '@/stores/dispatchStore'
import { Collections } from '@/utils/collections'
import { useDispatchPollAll } from '../hooks/useDispatchPollAll'
import { PhaseZeroSummary } from './PhaseZeroSummary'
import { DispatchAccordionRow } from './DispatchAccordionRow'
import { buildDispatchExport } from './exportDispatch'

/**
 * Dispatch tab content — PhaseZeroSummary + accordion rows for each dispatch.
 * Extracted from DispatchPanel so the panel can host multiple tabs.
 */
function DispatchTab() {
  const lastResponse = useStore(boardStore.store, boardStore.selectLastResponse)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const allWsStepIds = useStore(dispatchStore.store, dispatchStore.selectAllStepIds)
  const [copied, setCopied] = useState(false)

  const dispatches = useMemo(
    () => lastResponse?.dispatches ?? [],
    [lastResponse?.dispatches],
  )

  useDispatchPollAll(dispatches)

  const stepNameMap = useMemo(
    () => Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? s.id.slice(0, 8)),
    [steps],
  )

  // Step IDs discovered via WebSocket that weren't in the HTTP response (propagation dispatches)
  const httpStepIds = useMemo(
    () => new Set(dispatches.map((d) => d.step_id)),
    [dispatches],
  )
  const propagatedStepIds = useMemo(
    () => allWsStepIds.filter((id) => !httpStepIds.has(id)),
    [allWsStepIds, httpStepIds],
  )

  const hasDispatches = dispatches.length > 0 || propagatedStepIds.length > 0

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
      {!hasDispatches ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', py: 4 }}>
          <Typography variant="body2" sx={{ color: 'text.disabled', fontStyle: 'italic' }}>
            No dispatches yet. Submit the board to trigger dispatch agents.
          </Typography>
        </Box>
      ) : (
        <>
          <DispatchRows dispatches={dispatches} stepNameMap={stepNameMap} />
          {propagatedStepIds.map((stepId) => (
            <PropagatedRow
              key={stepId}
              stepId={stepId}
              stepName={stepNameMap.get(stepId) ?? stepId.slice(0, 8)}
            />
          ))}
        </>
      )}
    </>
  )
}

// ── Internal components (isolated store subscriptions per row) ───────────────

type DispatchRowsProps = {
  readonly dispatches: readonly { readonly execution_id: string; readonly step_id: string; readonly instruction: string }[]
  readonly stepNameMap: ReadonlyMap<string, string>
}

function DispatchRows({ dispatches, stepNameMap }: DispatchRowsProps) {
  return (
    <>
      {dispatches.map((d) => (
        <ConnectedRow
          key={d.execution_id}
          stepId={d.step_id}
          stepName={stepNameMap.get(d.step_id) ?? d.step_id.slice(0, 8)}
          instruction={d.instruction}
        />
      ))}
    </>
  )
}

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

type PropagatedRowProps = {
  readonly stepId: string
  readonly stepName: string
}

/** Render a dispatch row discovered via WebSocket (propagation re-design). */
function PropagatedRow({ stepId, stepName }: PropagatedRowProps) {
  const entry = useStore(dispatchStore.store, dispatchStore.selectByStepId(stepId))
  if (!entry) return null

  return (
    <DispatchAccordionRow
      stepName={stepName}
      instruction={entry.instruction}
      entry={entry}
    />
  )
}

export { DispatchTab }
