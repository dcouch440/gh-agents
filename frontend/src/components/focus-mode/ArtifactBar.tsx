import { useMemo, Fragment } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { FOCUS_MODE } from '@/constants'
import { ArtifactCard } from './ArtifactCard'
import type { ArtifactKind } from '@/stores'
import type { WorkflowStep, RosterAgent, RoomStepMember } from '@/types/workflow'

// ── Types ────────────────────────────────────────────────────────────────────

type ArtifactEntry = {
  id: string
  name: string
  subtitle: string | null
  kind: ArtifactKind
  stepId: string
  accentColor: string
}

type ArtifactBarProps = {
  steps: readonly WorkflowStep[]
  rosterByStep: Readonly<Record<string, readonly RosterAgent[]>>
  roomMembersByStep: Readonly<Record<string, readonly RoomStepMember[]>>
  currentStepId: string | null
  onArtifactClick: (id: string, kind: ArtifactKind) => void
}

// ── Constants ────────────────────────────────────────────────────────────────

const DOC_COLOR = '#D4793E'
const TASK_FORCE_COLOR = '#3b82f6'
const ROOM_COLOR = '#a78bfa'

// ── Component ────────────────────────────────────────────────────────────────

function ArtifactBar({ steps, rosterByStep, roomMembersByStep, currentStepId, onArtifactClick }: ArtifactBarProps) {
  const theme = useTheme()
  const documentDefsByStep = useStore(workflowStore.store, workflowStore.selectDocumentDefsByStep)

  const { documents, taskForces, rooms } = useMemo(() => {
    const docs: ArtifactEntry[] = []
    const tfs: ArtifactEntry[] = []
    const rms: ArtifactEntry[] = []

    for (const [stepId, defs] of Object.entries(documentDefsByStep)) {
      for (let i = 0; i < defs.length; i++) {
        const d = defs[i]!
        docs.push({
          id: d.id,
          name: d.name,
          subtitle: `~${d.target_length} chars`,
          kind: 'document',
          stepId,
          accentColor: DOC_COLOR,
        })
      }
    }

    for (let i = 0; i < steps.length; i++) {
      const step: WorkflowStep = steps[i]!
      if (step.execution_mode === 'task_force') {
        const roster = rosterByStep[step.id] ?? []
        tfs.push({
          id: step.id,
          name: step.name ?? 'Task Force',
          subtitle: roster.length > 0 ? `${roster.length} agent${roster.length > 1 ? 's' : ''}` : 'No agents',
          kind: 'task-force',
          stepId: step.id,
          accentColor: TASK_FORCE_COLOR,
        })
      }
      if (step.execution_mode === 'room') {
        const members = roomMembersByStep[step.id] ?? []
        rms.push({
          id: step.id,
          name: step.name ?? 'Room',
          subtitle: members.length > 0 ? `${members.length} member${members.length > 1 ? 's' : ''}` : 'No members',
          kind: 'room',
          stepId: step.id,
          accentColor: ROOM_COLOR,
        })
      }
    }

    return { documents: docs, taskForces: tfs, rooms: rms }
  }, [steps, documentDefsByStep, rosterByStep, roomMembersByStep])

  const hasDocuments = documents.length > 0
  const hasTaskForces = taskForces.length > 0
  const hasRooms = rooms.length > 0
  const hasAnyArtifacts = hasDocuments || hasTaskForces || hasRooms

  if (!hasAnyArtifacts) return null

  const groups: { label: string; entries: ArtifactEntry[] }[] = []
  if (hasDocuments) groups.push({ label: 'Docs', entries: documents })
  if (hasTaskForces) groups.push({ label: 'Teams', entries: taskForces })
  if (hasRooms) groups.push({ label: 'Rooms', entries: rooms })

  return (
    <Box
      sx={{
        height: FOCUS_MODE.ARTIFACT_BAR_HEIGHT,
        minHeight: FOCUS_MODE.ARTIFACT_BAR_HEIGHT,
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        pl: 1.5,
        pr: 6,
        py: 1,
        overflowX: 'auto',
        overflowY: 'hidden',
        borderBottom: 1,
        borderColor: 'divider',
        backgroundColor: theme.palette.custom.chromeBg,
        '&::-webkit-scrollbar': {
          height: 4,
        },
        '&::-webkit-scrollbar-thumb': {
          backgroundColor: theme.palette.divider,
          borderRadius: 2,
        },
      }}
    >
      {groups.map((group, gi) => (
        <Fragment key={group.label}>
          {gi > 0 && (
            <Box sx={{ width: 1, height: 48, backgroundColor: 'divider', flexShrink: 0 }} />
          )}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexShrink: 0 }}>
            <Typography
              sx={{
                fontSize: 9,
                fontWeight: 700,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'text.disabled',
                writingMode: 'vertical-rl',
                transform: 'rotate(180deg)',
                whiteSpace: 'nowrap',
              }}
            >
              {group.label}
            </Typography>
            {group.entries.map((entry) => (
              <ArtifactCard
                key={entry.id}
                name={entry.name}
                subtitle={entry.subtitle}
                accentColor={entry.accentColor}
                highlighted={entry.stepId === currentStepId}
                onClick={() => {
                  onArtifactClick(entry.id, entry.kind)
                }}
              />
            ))}
          </Box>
        </Fragment>
      ))}
    </Box>
  )
}

export { ArtifactBar }
export type { ArtifactBarProps, ArtifactEntry }
