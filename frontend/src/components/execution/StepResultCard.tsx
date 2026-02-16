import { useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import Collapse from '@mui/material/Collapse'
import Button from '@mui/material/Button'
import ExpandMoreOutlined from '@mui/icons-material/ExpandMoreOutlined'
import ExpandLessOutlined from '@mui/icons-material/ExpandLessOutlined'
import OpenInNewOutlined from '@mui/icons-material/OpenInNewOutlined'
import { MarkdownPreview } from '@/components/primitives'
import type { RunStepResult, PhaseExecution, ChildStepResult } from '@/types'

type StepResultCardProps = {
  step: RunStepResult
}

const STATUS_COLORS: Record<string, 'success' | 'error' | 'warning' | 'info' | 'default'> = {
  completed: 'success',
  complete: 'success',
  failed: 'error',
  running: 'warning',
  pending: 'info',
  skipped: 'default',
}

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60_000).toFixed(1)}m`
}

const formatTokens = (count: number): string => {
  if (count < 1000) return String(count)
  return `${(count / 1000).toFixed(1)}k`
}

const formatCost = (usd: number): string => {
  if (usd < 0.01) return `$${usd.toFixed(4)}`
  return `$${usd.toFixed(2)}`
}

function MetricChip({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: 'inline-flex', alignItems: 'center', gap: 0.5 }}>
      <Typography variant="caption" color="text.secondary">{label}</Typography>
      <Typography variant="caption" sx={{ fontWeight: 600 }}>{value}</Typography>
    </Box>
  )
}

function PhaseRow({ phase }: { phase: PhaseExecution }) {
  const [open, setOpen] = useState(false)
  const hasContent = phase.output_content !== null && phase.output_content.length > 0

  const phaseLabel = phase.document_name
    ? `${phase.phase}: ${phase.document_name}`
    : phase.phase

  return (
    <Box sx={{ border: 1, borderColor: 'divider', borderRadius: 1, mb: 0.5 }}>
      <Box
        onClick={() => { if (hasContent) setOpen((o) => !o) }}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 0.75,
          cursor: hasContent ? 'pointer' : 'default',
          '&:hover': hasContent ? { bgcolor: 'action.hover' } : undefined,
        }}
      >
        {hasContent && (
          open ? <ExpandLessOutlined sx={{ fontSize: 16 }} /> : <ExpandMoreOutlined sx={{ fontSize: 16 }} />
        )}
        <Typography variant="body2" sx={{ fontWeight: 500, flex: 1, textTransform: 'capitalize' }}>
          {phaseLabel}
        </Typography>
        <Chip
          label={phase.status}
          size="small"
          color={STATUS_COLORS[phase.status] ?? 'default'}
          sx={{ height: 20, fontSize: '0.7rem' }}
        />
        {phase.input_tokens !== null && phase.output_tokens !== null && (
          <MetricChip label="tok" value={`${formatTokens(phase.input_tokens)}/${formatTokens(phase.output_tokens)}`} />
        )}
        {phase.cost_usd !== null && (
          <MetricChip label="" value={formatCost(phase.cost_usd)} />
        )}
      </Box>
      {hasContent && (
        <Collapse in={open}>
          <Box sx={{ px: 1.5, py: 1, borderTop: 1, borderColor: 'divider', maxHeight: 300, overflow: 'auto' }}>
            <Typography
              variant="body2"
              component="pre"
              sx={{ fontFamily: 'monospace', fontSize: '0.75rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word', m: 0 }}
            >
              {phase.output_content}
            </Typography>
          </Box>
          {phase.error_message && (
            <Box sx={{ px: 1.5, py: 0.5, bgcolor: 'error.main', color: 'error.contrastText' }}>
              <Typography variant="caption">{phase.error_message}</Typography>
            </Box>
          )}
        </Collapse>
      )}
    </Box>
  )
}

function ChildStepRow({ childStep }: { childStep: ChildStepResult }) {
  return (
    <Box sx={{ border: 1, borderColor: 'divider', borderRadius: 1, mb: 0.5 }}>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 1.5,
          py: 0.75,
        }}
      >
        <Typography variant="body2" sx={{ fontWeight: 500, flex: 1 }}>
          {childStep.step_name ?? childStep.execution_mode}
        </Typography>
        <Chip
          label={childStep.execution_mode}
          size="small"
          variant="outlined"
          sx={{ height: 18, fontSize: '0.65rem' }}
        />
        <Chip
          label={childStep.status}
          size="small"
          color={STATUS_COLORS[childStep.status] ?? 'default'}
          sx={{ height: 20, fontSize: '0.7rem' }}
        />
        {childStep.input_tokens !== null && childStep.output_tokens !== null && (
          <MetricChip label="tok" value={`${formatTokens(childStep.input_tokens)}/${formatTokens(childStep.output_tokens)}`} />
        )}
        {childStep.duration_ms !== null && (
          <MetricChip label="" value={formatDuration(childStep.duration_ms)} />
        )}
      </Box>
      {childStep.error !== null && (
        <Box sx={{ px: 1.5, py: 0.5, borderTop: 1, borderColor: 'divider', bgcolor: 'rgba(248, 81, 73, 0.06)' }}>
          <Typography variant="caption" sx={{ color: '#f85149', fontFamily: 'monospace', fontSize: '0.7rem' }}>
            {childStep.error}
          </Typography>
        </Box>
      )}
    </Box>
  )
}

function StepResultCard({ step }: StepResultCardProps) {
  const { id: workflowId } = useParams<{ id: string }>()
  const [expanded, setExpanded] = useState(false)
  const hasChildSteps = step.child_steps !== null && step.child_steps.length > 0
  const hasOutput = step.output !== null || (step.phases !== null && step.phases.length > 0) || hasChildSteps
  const hasError = step.error !== null

  return (
    <Box
      sx={{
        border: 1,
        borderColor: 'divider',
        borderRadius: 2,
        overflow: 'hidden',
        mb: 1.5,
      }}
    >
      {/* Header */}
      <Box
        onClick={() => { if (hasOutput || hasError) setExpanded((o) => !o) }}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 2,
          py: 1.25,
          cursor: hasOutput || hasError ? 'pointer' : 'default',
          '&:hover': hasOutput || hasError ? { bgcolor: 'action.hover' } : undefined,
        }}
      >
        <Typography variant="body2" sx={{ fontWeight: 600, flex: 1 }}>
          {step.step_name ?? step.step_id}
        </Typography>
        <Chip
          label={step.execution_mode}
          size="small"
          variant="outlined"
          sx={{ height: 20, fontSize: '0.65rem' }}
        />
        <Chip
          label={step.status}
          size="small"
          color={STATUS_COLORS[step.status] ?? 'default'}
          sx={{ height: 22 }}
        />
        {(hasOutput || hasError) && (
          expanded ? <ExpandLessOutlined fontSize="small" /> : <ExpandMoreOutlined fontSize="small" />
        )}
      </Box>

      {/* Metrics row */}
      <Box sx={{ display: 'flex', gap: 2, px: 2, pb: 1, flexWrap: 'wrap' }}>
        {step.duration_ms !== null && (
          <MetricChip label="Duration" value={formatDuration(step.duration_ms)} />
        )}
        {step.input_tokens !== null && step.output_tokens !== null && (
          <MetricChip label="Tokens" value={`${formatTokens(step.input_tokens)} in / ${formatTokens(step.output_tokens)} out`} />
        )}
        {step.cost_usd !== null && (
          <MetricChip label="Cost" value={formatCost(step.cost_usd)} />
        )}
      </Box>

      {/* Expanded content */}
      <Collapse in={expanded}>
        <Box sx={{ px: 2, pb: 1.5, borderTop: 1, borderColor: 'divider' }}>
          {hasError && (
            <Box
              sx={{
                mt: 1,
                p: 1.5,
                borderRadius: 1,
                backgroundColor: 'rgba(248, 81, 73, 0.08)',
                border: '1px solid rgba(248, 81, 73, 0.2)',
              }}
            >
              <Typography
                variant="body2"
                sx={{
                  color: '#f85149',
                  fontFamily: 'monospace',
                  fontSize: '0.8125rem',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                }}
              >
                {step.error}
              </Typography>
            </Box>
          )}

          {step.output !== null && (
            <Box sx={{ mt: 1, maxHeight: 300, overflow: 'auto', borderRadius: 1, border: '1px solid', borderColor: 'divider', p: 1.5 }}>
              <MarkdownPreview content={step.output} />
            </Box>
          )}

          {step.structured_output !== null && (
            <Box sx={{ mt: 1 }}>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                Structured Output
              </Typography>
              <Box sx={{ bgcolor: 'action.hover', borderRadius: 1, p: 1, maxHeight: 200, overflow: 'auto' }}>
                <Typography
                  variant="body2"
                  component="pre"
                  sx={{ fontFamily: 'monospace', fontSize: '0.75rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word', m: 0 }}
                >
                  {JSON.stringify(step.structured_output, null, 2)}
                </Typography>
              </Box>
            </Box>
          )}

          {step.phases !== null && step.phases.length > 0 && (
            <Box sx={{ mt: 1 }}>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                Pipeline Phases ({step.phases.length})
              </Typography>
              {step.phases.map((phase) => (
                <PhaseRow key={phase.id} phase={phase} />
              ))}
            </Box>
          )}

          {hasChildSteps && (
            <Box sx={{ mt: 1 }}>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                Child Steps ({step.child_steps!.length})
              </Typography>
              {step.child_steps!.map((cs, idx) => (
                <ChildStepRow key={idx} childStep={cs} />
              ))}
              {step.child_execution_id !== null && workflowId && (
                <Box sx={{ mt: 1, display: 'flex', justifyContent: 'flex-end' }}>
                  <Button
                    component={Link}
                    to={`/workflows/${workflowId}/runs/${step.child_execution_id}`}
                    size="small"
                    variant="text"
                    endIcon={<OpenInNewOutlined sx={{ fontSize: 14 }} />}
                    sx={{ fontSize: '0.75rem', textTransform: 'none' }}
                  >
                    View Full Run
                  </Button>
                </Box>
              )}
            </Box>
          )}
        </Box>
      </Collapse>
    </Box>
  )
}

export { StepResultCard }
export type { StepResultCardProps }
