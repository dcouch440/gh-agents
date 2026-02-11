import { useEffect, useMemo, useCallback, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Box, Button, Typography, TextField } from '@mui/material'
import AddIcon from '@mui/icons-material/Add'
import DeleteIcon from '@mui/icons-material/Delete'
import EditIcon from '@mui/icons-material/Edit'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import AccountTreeOutlined from '@mui/icons-material/AccountTreeOutlined'
import { FadeIn } from '@/components/animation'
import { PageHeader, Table, ConfirmModal, EmptyState, type TableColumn, type MenuAction } from '@/components/primitives'
import { ActionMenu } from '@/components/primitives'
import { useConfirmModal } from '@/hooks/useConfirmModal'
import { useStore, workflowStore } from '@/stores'
import { ANIMATION } from '@/constants'
import { formatRelativeTime } from '@/utils/formatRelativeTime'
import type { Workflow } from '@/types/workflow'

function WorkflowsPage() {
  const navigate = useNavigate()
  const workflows = useStore(workflowStore.store, workflowStore.selectAll)
  const loading = useStore(workflowStore.store, workflowStore.selectLoading)
  const error = useStore(workflowStore.store, workflowStore.selectError)
  const confirmModal = useConfirmModal()
  const { openConfirm } = confirmModal
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')

  useEffect(() => {
    void workflowStore.fetchAll()
  }, [])

  const handleCreate = useCallback(async () => {
    if (!newName.trim()) return
    try {
      const workflow = await workflowStore.create({ name: newName.trim() })
      setNewName('')
      setCreating(false)
      void navigate(`/workflows/${workflow.id}`)
    } catch {
      // error handled by store
    }
  }, [newName, navigate])

  const handleDelete = useCallback(
    (workflow: Workflow) => {
      openConfirm({
        title: 'Delete Workflow',
        message: `Are you sure you want to delete "${workflow.name}"? This action cannot be undone.`,
        confirmText: 'Delete',
        confirmColor: 'error',
        onConfirm: async () => {
          await workflowStore.remove(workflow.id)
        },
      })
    },
    [openConfirm],
  )

  const columns: TableColumn<Workflow>[] = useMemo(
    () => [
      {
        key: 'name',
        header: 'Name',
        sortable: true,
        width: 280,
        render: (wf) => (
          <Typography variant="body2" fontWeight={500}>
            {wf.name}
          </Typography>
        ),
      },
      {
        key: 'description',
        header: 'Description',
        truncate: true,
        width: 400,
        render: (wf) => (
          <Typography variant="body2" color="text.secondary">
            {wf.description ?? 'No description'}
          </Typography>
        ),
      },
      {
        key: 'created_at',
        header: 'Created',
        sortable: true,
        width: 140,
        render: (wf) => (
          <Typography variant="body2" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
            {formatRelativeTime(wf.created_at)}
          </Typography>
        ),
      },
      {
        key: 'actions',
        header: 'Actions',
        width: 80,
        align: 'center' as const,
        render: (wf) => {
          const actions: MenuAction[] = [
            {
              key: 'open',
              label: 'Open Editor',
              icon: <OpenInNewIcon fontSize="small" />,
              onClick: () => {
                void navigate(`/workflows/${wf.id}`)
              },
              dividerAfter: true,
            },
            {
              key: 'edit',
              label: 'Edit Details',
              icon: <EditIcon fontSize="small" />,
              onClick: () => {
                void navigate(`/workflows/${wf.id}`)
              },
            },
            {
              key: 'delete',
              label: 'Delete',
              icon: <DeleteIcon fontSize="small" />,
              onClick: () => {
                void handleDelete(wf)
              },
              color: 'error' as const,
            },
          ]

          return <ActionMenu actions={actions} ariaLabel={`Actions for ${wf.name}`} />
        },
      },
    ],
    [navigate, handleDelete],
  )

  const isEmpty = !loading && workflows.length === 0 && !error

  return (
    <FadeIn>
      <Box>
        <PageHeader title="Workflows" description="Build and manage AI workflow pipelines.">
          {creating ? (
            <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
              <TextField
                size="small"
                placeholder="Workflow name..."
                value={newName}
                onChange={(e) => {
                  setNewName(e.target.value)
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleCreate()
                  if (e.key === 'Escape') setCreating(false)
                }}
                autoFocus
                sx={{ width: 220 }}
              />
              <Button
                variant="contained"
                size="small"
                onClick={() => {
                  void handleCreate()
                }}
                disabled={!newName.trim()}
              >
                Create
              </Button>
              <Button
                variant="outlined"
                size="small"
                onClick={() => {
                  setCreating(false)
                  setNewName('')
                }}
                sx={{ transition: `all ${ANIMATION.FAST}ms ease` }}
              >
                Cancel
              </Button>
            </Box>
          ) : (
            <Button
              variant="contained"
              startIcon={<AddIcon />}
              onClick={() => {
                setCreating(true)
              }}
            >
              New Workflow
            </Button>
          )}
        </PageHeader>

        {isEmpty ? (
          <EmptyState
            icon={<AccountTreeOutlined sx={{ fontSize: 48 }} />}
            title="No workflows yet"
            description="Create your first workflow to start building AI pipelines."
            action={
              <Button
                variant="contained"
                startIcon={<AddIcon />}
                onClick={() => {
                  setCreating(true)
                }}
              >
                New Workflow
              </Button>
            }
          />
        ) : (
          <Table
            data={workflows}
            keyExtractor={(wf) => wf.id}
            columns={columns}
            loading={loading}
            error={error}
            emptyMessage="No workflows found."
            enableSorting
            enableSearch
            searchPlaceholder="Search workflows..."
            searchFields={['name', 'description']}
            defaultSortColumn="created_at"
            defaultSortDirection="desc"
            defaultPageSize={25}
            pageSizeOptions={[10, 25, 50]}
            onRowClick={(wf) => {
              void navigate(`/workflows/${wf.id}`)
            }}
            stickyHeader
            density="normal"
          />
        )}

        <ConfirmModal
          open={confirmModal.open}
          onClose={confirmModal.closeConfirm}
          onConfirm={confirmModal.handleConfirm}
          title={confirmModal.title}
          message={confirmModal.message}
          confirmText={confirmModal.confirmText}
          cancelText={confirmModal.cancelText}
          confirmColor={confirmModal.confirmColor}
          loading={confirmModal.loading}
          error={confirmModal.error}
        />
      </Box>
    </FadeIn>
  )
}

export { WorkflowsPage }
