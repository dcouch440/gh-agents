import { useEffect, useCallback, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import CircularProgress from '@mui/material/CircularProgress'
import IconButton from '@mui/material/IconButton'
import MuiTooltip from '@mui/material/Tooltip'
import ArrowBackOutlined from '@mui/icons-material/ArrowBackOutlined'
import RefreshOutlined from '@mui/icons-material/RefreshOutlined'
import RestoreOutlined from '@mui/icons-material/RestoreOutlined'
import AssignmentOutlined from '@mui/icons-material/AssignmentOutlined'
import { FadeIn } from '@/components/animation'
import { PageHeader, EmptyState, ConfirmModal } from '@/components/primitives'
import { useConfirmModal } from '@/hooks'
import { StepResultCard } from '@/components/execution/StepResultCard'
import { api } from '@/api'
import type { Workflow, RunDetailResponse } from '@/types'

const STATUS_COLORS: Record<string, 'success' | 'error' | 'warning' | 'info' | 'default'> = {
  completed: 'success',
  failed: 'error',
  running: 'warning',
  pending: 'info',
}

const formatDuration = (ms: number | null): string => {
  if (ms === null) return '-'
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

function RunDetailPage() {
  const { id: workflowId, runId } = useParams<{ id: string; runId: string }>()
  const navigate = useNavigate()
  const [workflow, setWorkflow] = useState<Workflow | null>(null)
  const [detail, setDetail] = useState<RunDetailResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const confirm = useConfirmModal()

  const handleRebase = useCallback(() => {
    if (!workflowId || !detail?.execution.template_id) return
    const templateId = detail.execution.template_id
    const templateName = detail.template_name ?? 'unknown'
    confirm.openConfirm({
      title: 'Rebase Workshop',
      message: `This will overwrite your current workshop configuration with the template "${templateName}" used for this run. Your current state will be auto-saved as a backup template.`,
      confirmText: 'Rebase',
      confirmColor: 'warning',
      onConfirm: async () => {
        await api.workflows.rebase(workflowId, { template_id: templateId })
        void navigate(`/workflows/${workflowId}`)
      },
    })
  }, [workflowId, detail, confirm, navigate])

  const fetchData = useCallback(async () => {
    if (!workflowId || !runId) return
    setLoading(true)
    setError(null)
    try {
      const [wf, runDetail] = await Promise.all([
        api.workflows.get(workflowId),
        api.workflows.getRunDetail(workflowId, runId),
      ])
      setWorkflow(wf)
      setDetail(runDetail)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load run detail')
    } finally {
      setLoading(false)
    }
  }, [workflowId, runId])

  useEffect(() => {
    void fetchData()
  }, [fetchData])

  if (!workflowId || !runId) return null

  return (
    <FadeIn>
      <Box>
        <PageHeader
          title={workflow ? `${workflow.name} - Run Detail` : 'Run Detail'}
          description={detail?.template_name ? `Template: ${detail.template_name}` : 'Per-step execution results for this run.'}
          breadcrumbs={[
            { label: 'Workflows', path: '/workflows' },
            ...(workflow ? [{ label: workflow.name, path: `/workflows/${workflowId}` }] : []),
            { label: 'Runs', path: `/workflows/${workflowId}/runs` },
            { label: runId.slice(0, 8) },
          ]}
        >
          <Box sx={{ display: 'flex', gap: 1 }}>
            <MuiTooltip title="Back to run history">
              <IconButton
                size="small"
                onClick={() => { void navigate(`/workflows/${workflowId}/runs`) }}
              >
                <ArrowBackOutlined fontSize="small" />
              </IconButton>
            </MuiTooltip>
            <MuiTooltip title="Refresh">
              <IconButton size="small" onClick={() => { void fetchData() }} disabled={loading}>
                <RefreshOutlined fontSize="small" />
              </IconButton>
            </MuiTooltip>
            {detail?.execution.template_id && (
              <MuiTooltip title="Rebase workshop to this template">
                <IconButton size="small" onClick={handleRebase}>
                  <RestoreOutlined fontSize="small" />
                </IconButton>
              </MuiTooltip>
            )}
          </Box>
        </PageHeader>

        {loading && detail === null ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }}>
            <CircularProgress size={32} />
          </Box>
        ) : error ? (
          <Box sx={{ py: 4, textAlign: 'center' }}>
            <Typography color="error">{error}</Typography>
          </Box>
        ) : detail === null ? (
          <EmptyState
            icon={<AssignmentOutlined sx={{ fontSize: 48 }} />}
            title="Run not found"
            description="This execution does not exist or has been deleted."
          />
        ) : (
          <Box sx={{ px: 2 }}>
            {/* Summary header card */}
            <Box
              sx={{
                border: 1,
                borderColor: 'divider',
                borderRadius: 2,
                p: 2,
                mb: 2,
                display: 'flex',
                alignItems: 'center',
                gap: 2,
                flexWrap: 'wrap',
              }}
            >
              <Chip
                label={detail.execution.status}
                color={STATUS_COLORS[detail.execution.status] ?? 'default'}
                sx={{ fontWeight: 600 }}
              />
              {detail.execution.execution_mode && (
                <Chip label={detail.execution.execution_mode} size="small" variant="outlined" />
              )}
              {detail.template_name && (
                <Chip label={detail.template_name} size="small" variant="outlined" color="info" />
              )}

              <Box sx={{ display: 'flex', gap: 3, ml: 'auto', flexWrap: 'wrap' }}>
                <Box>
                  <Typography variant="caption" color="text.secondary">Duration</Typography>
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>{formatDuration(detail.duration_ms)}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Tokens</Typography>
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>
                    {formatTokens(detail.total_input_tokens)} in / {formatTokens(detail.total_output_tokens)} out
                  </Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Cost</Typography>
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>{formatCost(detail.total_cost_usd)}</Typography>
                </Box>
              </Box>
            </Box>

            {/* Error banner */}
            {detail.execution.error && (
              <Box
                sx={{
                  mb: 2,
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
                  {detail.execution.error}
                </Typography>
              </Box>
            )}

            {/* Step results */}
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1 }}>
              Steps ({detail.steps.length})
            </Typography>
            {detail.steps.length === 0 ? (
              <Typography variant="body2" color="text.disabled" sx={{ fontStyle: 'italic' }}>
                No step execution data available.
              </Typography>
            ) : (
              detail.steps.map((step) => (
                <StepResultCard key={step.step_id} step={step} />
              ))
            )}
          </Box>
        )}
      </Box>
      <ConfirmModal
        open={confirm.open}
        onClose={confirm.closeConfirm}
        onConfirm={confirm.handleConfirm}
        title={confirm.title}
        message={confirm.message}
        confirmText={confirm.confirmText}
        cancelText={confirm.cancelText}
        confirmColor={confirm.confirmColor}
        loading={confirm.loading}
        error={confirm.error}
      />
    </FadeIn>
  )
}

export { RunDetailPage }
