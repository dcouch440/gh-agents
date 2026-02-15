import { useEffect, useMemo, useCallback, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import CircularProgress from '@mui/material/CircularProgress'
import IconButton from '@mui/material/IconButton'
import MuiTooltip from '@mui/material/Tooltip'
import ArrowBackOutlined from '@mui/icons-material/ArrowBackOutlined'
import RefreshOutlined from '@mui/icons-material/RefreshOutlined'
import HistoryOutlined from '@mui/icons-material/HistoryOutlined'
import { FadeIn } from '@/components/animation'
import { PageHeader, Table, EmptyState, type TableColumn } from '@/components/primitives'
import { api } from '@/api'
import { formatRelativeTime } from '@/utils/formatRelativeTime'
import type { Workflow, WorkflowExecutionSummary } from '@/types'

const STATUS_COLORS: Record<string, 'success' | 'error' | 'warning' | 'info' | 'default'> = {
  completed: 'success',
  failed: 'error',
  running: 'warning',
  pending: 'info',
}

const formatDuration = (startedAt: string | null, completedAt: string | null): string | null => {
  if (!startedAt || !completedAt) return null
  const ms = new Date(completedAt).getTime() - new Date(startedAt).getTime()
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60_000).toFixed(1)}m`
}

function RunHistoryPage() {
  const { id: workflowId } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [workflow, setWorkflow] = useState<Workflow | null>(null)
  const [runs, setRuns] = useState<WorkflowExecutionSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    if (!workflowId) return
    setLoading(true)
    setError(null)
    try {
      const [wf, executions] = await Promise.all([
        api.workflows.get(workflowId),
        api.workflows.listExecutions(workflowId),
      ])
      setWorkflow(wf)
      setRuns(executions)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load data')
    } finally {
      setLoading(false)
    }
  }, [workflowId])

  useEffect(() => {
    void fetchData()
  }, [fetchData])

  const columns: TableColumn<WorkflowExecutionSummary>[] = useMemo(
    () => [
      {
        key: 'status',
        header: 'Status',
        width: 100,
        render: (run) => (
          <Chip
            label={run.status}
            size="small"
            color={STATUS_COLORS[run.status] ?? 'default'}
            sx={{ height: 22 }}
          />
        ),
      },
      {
        key: 'execution_mode',
        header: 'Mode',
        width: 100,
        render: (run) => (
          <Chip label={run.execution_mode} size="small" variant="outlined" sx={{ height: 20, fontSize: '0.65rem' }} />
        ),
      },
      {
        key: 'started_at',
        header: 'Started',
        sortable: true,
        width: 160,
        render: (run) => (
          <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
            {formatRelativeTime(run.started_at)}
          </Typography>
        ),
      },
      {
        key: 'duration',
        header: 'Duration',
        width: 100,
        render: (run) => {
          const duration = formatDuration(run.started_at, run.completed_at)
          return (
            <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
              {duration ?? '-'}
            </Typography>
          )
        },
      },
      {
        key: 'id',
        header: 'Run ID',
        width: 200,
        render: (run) => (
          <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.7rem' }}>
            {run.id.slice(0, 8)}
          </Typography>
        ),
      },
    ],
    [],
  )

  if (!workflowId) return null

  return (
    <FadeIn>
      <Box>
        <PageHeader
          title={workflow ? `${workflow.name} - Run History` : 'Run History'}
          description="View all workflow executions and drill into per-step results."
          breadcrumbs={[
            { label: 'Workflows', path: '/workflows' },
            ...(workflow ? [{ label: workflow.name, path: `/workflows/${workflowId}` }] : []),
            { label: 'Runs' },
          ]}
        >
          <Box sx={{ display: 'flex', gap: 1 }}>
            <MuiTooltip title="Back to editor">
              <IconButton
                size="small"
                onClick={() => { void navigate(`/workflows/${workflowId}`) }}
              >
                <ArrowBackOutlined fontSize="small" />
              </IconButton>
            </MuiTooltip>
            <MuiTooltip title="Refresh">
              <IconButton size="small" onClick={() => { void fetchData() }} disabled={loading}>
                <RefreshOutlined fontSize="small" />
              </IconButton>
            </MuiTooltip>
          </Box>
        </PageHeader>

        {loading && runs.length === 0 ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }}>
            <CircularProgress size={32} />
          </Box>
        ) : error ? (
          <Box sx={{ py: 4, textAlign: 'center' }}>
            <Typography color="error">{error}</Typography>
          </Box>
        ) : runs.length === 0 ? (
          <EmptyState
            icon={<HistoryOutlined sx={{ fontSize: 48 }} />}
            title="No runs yet"
            description="Run the workflow to see execution history."
          />
        ) : (
          <Table
            data={runs}
            keyExtractor={(run) => run.id}
            columns={columns}
            loading={loading}
            emptyMessage="No runs found."
            defaultSortColumn="started_at"
            defaultSortDirection="desc"
            defaultPageSize={25}
            pageSizeOptions={[10, 25, 50]}
            onRowClick={(run) => {
              void navigate(`/workflows/${workflowId}/runs/${run.id}`)
            }}
            stickyHeader
            density="normal"
          />
        )}
      </Box>
    </FadeIn>
  )
}

export { RunHistoryPage }
