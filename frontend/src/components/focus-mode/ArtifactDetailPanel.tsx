import { useState, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import CloseOutlined from '@mui/icons-material/CloseOutlined'
import Chip from '@mui/material/Chip'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { ANIMATION } from '@/constants'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import type { ArtifactKind } from '@/stores'
import type { DocumentDef, RosterAgent, RoomStepMember, WorkflowStep } from '@/types/workflow'

type ArtifactDetailPanelProps = {
  artifactId: string
  artifactKind: ArtifactKind
  onClose: () => void
}

function ArtifactDetailPanel({ artifactId, artifactKind, onClose }: ArtifactDetailPanelProps) {
  if (artifactKind === 'input' || artifactKind === 'context') {
    return <StepContentDetail stepId={artifactId} kind={artifactKind} onClose={onClose} />
  }
  if (artifactKind === 'document') {
    return <DocumentDetail artifactId={artifactId} onClose={onClose} />
  }
  if (artifactKind === 'roster-agent') {
    return <AgentDetail artifactId={artifactId} onClose={onClose} />
  }
  if (artifactKind === 'room-member') {
    return <MemberDetail artifactId={artifactId} onClose={onClose} />
  }
  if (artifactKind === 'task-force') {
    return <TaskForceDetail stepId={artifactId} onClose={onClose} />
  }
  return <RoomDetail stepId={artifactId} onClose={onClose} />
}

// ── Sub-components ───────────────────────────────────────────────────────────

type DetailShellProps = {
  title: string
  accentColor: string
  onClose: () => void
  children: React.ReactNode
}

function DetailShell({ title, accentColor, onClose, children }: DetailShellProps) {
  const theme = useTheme()

  return (
    <Box
      sx={{
        position: 'absolute',
        inset: 0,
        zIndex: 2,
        display: 'flex',
        flexDirection: 'column',
        backgroundColor: theme.palette.background.default,
        animation: `slideDown ${ANIMATION.NORMAL}ms ease`,
        '@keyframes slideDown': {
          from: { opacity: 0, transform: 'translateY(-16px)' },
          to: { opacity: 1, transform: 'translateY(0)' },
        },
      }}
    >
      {/* Header */}
      <Box
        sx={{
          height: 52,
          display: 'flex',
          alignItems: 'center',
          gap: 1.5,
          px: 2,
          borderBottom: 1,
          borderColor: 'divider',
          backgroundColor: theme.palette.custom.bgHeader,
          flexShrink: 0,
        }}
      >
        <Box sx={{ width: 4, height: 24, borderRadius: 2, backgroundColor: accentColor, flexShrink: 0 }} />
        <Typography sx={{ fontSize: 14, fontWeight: 600, color: 'text.primary', flex: 1, minWidth: 0 }}>
          {title}
        </Typography>
        <IconButton onClick={onClose} size="small" sx={{ width: 32, height: 32, color: 'text.secondary' }}>
          <CloseOutlined sx={{ fontSize: 18 }} />
        </IconButton>
      </Box>

      {/* Content */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 2.5 }}>
        {children}
      </Box>
    </Box>
  )
}

// ── Document Detail ──────────────────────────────────────────────────────────

function DocumentDetail({ artifactId, onClose }: { artifactId: string; onClose: () => void }) {
  const documentDefsByStep = useStore(workflowStore.store, workflowStore.selectDocumentDefsByStep)
  const contentByDefId = useStore(workflowStore.store, workflowStore.selectDocumentContentByDefId)

  let doc: DocumentDef | null = null
  for (const defs of Object.values(documentDefsByStep)) {
    const found = defs.find((d) => d.id === artifactId)
    if (found) {
      doc = found
      break
    }
  }

  if (!doc) {
    return (
      <DetailShell title="Document" accentColor="#D4793E" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Document not found</Typography>
      </DetailShell>
    )
  }

  const content = contentByDefId[artifactId] ?? null

  return (
    <DetailShell title={doc.name} accentColor="#D4793E" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {doc.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {doc.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`Target: ~${doc.target_length} chars`} size="small" variant="outlined" />
          {doc.document_id !== null && <Chip label="Generated" size="small" color="success" variant="outlined" />}
        </Box>
        {content !== null ? (
          <Box
            sx={{
              p: 2,
              borderRadius: '8px',
              border: 1,
              borderColor: 'divider',
              backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
              whiteSpace: 'pre-wrap',
              fontSize: 13,
              lineHeight: 1.6,
              color: 'text.primary',
              fontFamily: 'monospace',
            }}
          >
            {content}
          </Box>
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No content generated yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

// ── Agent Detail ─────────────────────────────────────────────────────────────

function AgentDetail({ artifactId, onClose }: { artifactId: string; onClose: () => void }) {
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)

  let agent: RosterAgent | null = null
  for (const roster of Object.values(rosterByStep)) {
    const found = roster.find((a) => a.id === artifactId)
    if (found) {
      agent = found
      break
    }
  }

  if (!agent) {
    return (
      <DetailShell title="Agent" accentColor="#3b82f6" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Agent not found</Typography>
      </DetailShell>
    )
  }

  return (
    <DetailShell title={agent.name} accentColor="#3b82f6" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {agent.role_description && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Role
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {agent.role_description}
            </Typography>
          </Box>
        )}
        {agent.capabilities.length > 0 && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Capabilities
            </Typography>
            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
              {agent.capabilities.map((cap, i) => (
                <Chip key={i} label={cap} size="small" variant="outlined" />
              ))}
            </Box>
          </Box>
        )}
      </Box>
    </DetailShell>
  )
}

// ── Member Detail ────────────────────────────────────────────────────────────

function MemberDetail({ artifactId, onClose }: { artifactId: string; onClose: () => void }) {
  const roomMembersByStep = useStore(workflowStore.store, workflowStore.selectRoomMembersByStep)

  let member: RoomStepMember | null = null
  for (const members of Object.values(roomMembersByStep)) {
    const found = members.find((m) => m.id === artifactId)
    if (found) {
      member = found
      break
    }
  }

  if (!member) {
    return (
      <DetailShell title="Member" accentColor="#a78bfa" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Member not found</Typography>
      </DetailShell>
    )
  }

  return (
    <DetailShell title={member.name} accentColor="#a78bfa" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {member.role && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Role
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {member.role}
            </Typography>
          </Box>
        )}
        {member.perspective && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Perspective
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {member.perspective}
            </Typography>
          </Box>
        )}
      </Box>
    </DetailShell>
  )
}

// ── Task Force Detail ────────────────────────────────────────────────────────

function TaskForceDetail({ stepId, onClose }: { stepId: string; onClose: () => void }) {
  const step: WorkflowStep | null = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const roster = rosterByStep[stepId] ?? []

  const title = step?.name ?? 'Task Force'

  return (
    <DetailShell title={title} accentColor="#3b82f6" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {step?.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {step.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`${roster.length} agent${roster.length !== 1 ? 's' : ''}`} size="small" variant="outlined" />
        </Box>
        {roster.length > 0 ? (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
            {roster.map((agent: RosterAgent) => (
              <Box
                key={agent.id}
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  border: 1,
                  borderColor: 'divider',
                  backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 0.5,
                }}
              >
                <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>
                  {agent.name}
                </Typography>
                {agent.role_description && (
                  <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
                    {agent.role_description}
                  </Typography>
                )}
                {agent.capabilities.length > 0 && (
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mt: 0.5 }}>
                    {agent.capabilities.map((cap, i) => (
                      <Chip key={i} label={cap} size="small" variant="outlined" sx={{ fontSize: 10 }} />
                    ))}
                  </Box>
                )}
              </Box>
            ))}
          </Box>
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No agents in roster yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

// ── Room Detail ─────────────────────────────────────────────────────────────

function RoomDetail({ stepId, onClose }: { stepId: string; onClose: () => void }) {
  const step: WorkflowStep | null = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const roomMembersByStep = useStore(workflowStore.store, workflowStore.selectRoomMembersByStep)
  const members = roomMembersByStep[stepId] ?? []

  const title = step?.name ?? 'Room'

  return (
    <DetailShell title={title} accentColor="#a78bfa" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {step?.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {step.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`${members.length} member${members.length !== 1 ? 's' : ''}`} size="small" variant="outlined" />
        </Box>
        {members.length > 0 ? (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
            {members.map((member: RoomStepMember) => (
              <Box
                key={member.id}
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  border: 1,
                  borderColor: 'divider',
                  backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 0.5,
                }}
              >
                <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>
                  {member.name}
                </Typography>
                {member.role && (
                  <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
                    {member.role}
                  </Typography>
                )}
                {member.perspective && (
                  <Typography sx={{ fontSize: 11, color: 'text.disabled', lineHeight: 1.4, fontStyle: 'italic' }}>
                    {member.perspective}
                  </Typography>
                )}
              </Box>
            ))}
          </Box>
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No members added yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

// ── Step Content Detail (Input / Context) ───────────────────────────────────

type ContentViewMode = 'raw' | 'md'

function StepContentDetail({ stepId, kind, onClose }: { stepId: string; kind: 'input' | 'context'; onClose: () => void }) {
  const step: WorkflowStep | null = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const [viewMode, setViewMode] = useState<ContentViewMode>('raw')

  const accentColor = STEP_TYPE_COLORS[kind] ?? DEFAULT_STEP_TYPE_COLOR
  const title = step?.name ?? (kind === 'input' ? 'Input' : 'Context')
  const content = step?.prompt_template ?? ''

  const handleChange = useCallback((value: string) => {
    workflowStore.patchStepLocal(stepId, { prompt_template: value })
  }, [stepId])

  return (
    <DetailShell title={title} accentColor={accentColor} onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 1 }}>
        {/* Raw / Md toggle */}
        <Box sx={{ display: 'flex', gap: 0.25, justifyContent: 'flex-end' }}>
          {(['raw', 'md'] as const).map((vm) => (
            <Box
              key={vm}
              onClick={() => {
                setViewMode(vm)
              }}
              sx={{
                px: 0.75,
                py: 0.25,
                borderRadius: '4px',
                fontSize: 10,
                fontWeight: 600,
                cursor: 'pointer',
                userSelect: 'none',
                color: viewMode === vm ? accentColor : 'text.disabled',
                backgroundColor: viewMode === vm ? `${accentColor}15` : 'transparent',
                transition: 'all 120ms ease',
                '&:hover': viewMode === vm ? {} : { color: 'text.secondary' },
              }}
            >
              {vm === 'raw' ? 'Raw' : 'Md'}
            </Box>
          ))}
        </Box>

        {/* Content area */}
        <Box sx={{ flex: 1, overflow: 'hidden' }}>
          {viewMode === 'raw' ? (
            <CodeEditor
              value={content}
              onChange={handleChange}
              placeholder={kind === 'input' ? 'Type your input here...' : 'Type your context here...'}
              height="100%"
            />
          ) : (
            <Box sx={{ overflow: 'auto', height: '100%' }}>
              <MarkdownPreview content={content} />
            </Box>
          )}
        </Box>
      </Box>
    </DetailShell>
  )
}

export { ArtifactDetailPanel }
export type { ArtifactDetailPanelProps }
